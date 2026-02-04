"""
Comp-AI API client for Airflow DAGs.

Uses Airflow Variables: COMP_AI_BASE_URL, COMP_AI_TOKEN.
Optional: COMP_AI_KEYCLOAK_URL, COMP_AI_CLIENT_ID, COMP_AI_CLIENT_SECRET, COMP_AI_KEYCLOAK_REALM
  — if set, fetches a fresh token via client_credentials (avoids 401 from expired token).
Used by comp_ai_control_tests_dag and comp_ai_alerts_dag.
"""
from __future__ import annotations

import logging
import os
from typing import Any, Optional

import requests

log = logging.getLogger(__name__)


def _ascii_safe(s: str) -> str:
    """Strip non-ASCII so HTTP headers (latin-1) don't raise UnicodeEncodeError."""
    ascii_only = s.encode("ascii", "ignore").decode("ascii")
    if len(ascii_only) != len(s):
        log.warning(
            "COMP_AI_TOKEN or COMP_AI_BASE_URL contained non-ASCII characters; "
            "ensure Variable values use only ASCII (e.g. no ellipsis …)."
        )
    return ascii_only


def _get_variable(key: str, default: Optional[str] = None) -> Optional[str]:
    try:
        from airflow.sdk import Variable
        return Variable.get(key, default=default)
    except Exception:
        return default


def _fetch_token_client_credentials() -> Optional[str]:
    """Fetch Bearer token from Keycloak (client_credentials). Returns None if config missing."""
    keycloak_url = os.environ.get("COMP_AI_KEYCLOAK_URL") or _get_variable("COMP_AI_KEYCLOAK_URL")
    client_id = os.environ.get("COMP_AI_CLIENT_ID") or _get_variable("COMP_AI_CLIENT_ID")
    client_secret = os.environ.get("COMP_AI_CLIENT_SECRET") or _get_variable("COMP_AI_CLIENT_SECRET")
    realm = os.environ.get("COMP_AI_KEYCLOAK_REALM") or _get_variable("COMP_AI_KEYCLOAK_REALM") or "lianel"
    if not keycloak_url or not client_id or not client_secret:
        return None
    keycloak_url = keycloak_url.rstrip("/")
    token_url = f"{keycloak_url}/realms/{realm}/protocol/openid-connect/token"
    try:
        resp = requests.post(
            token_url,
            data={
                "grant_type": "client_credentials",
                "client_id": client_id,
                "client_secret": client_secret,
            },
            headers={"Content-Type": "application/x-www-form-urlencoded"},
            timeout=15,
        )
        resp.raise_for_status()
        data = resp.json()
        token = (data.get("access_token") or "").strip()
        if token:
            return _ascii_safe(token)
    except Exception as e:
        log.warning("Failed to fetch token via client_credentials: %s", e)
    return None


def _get_config() -> tuple[str, str]:
    """Get base URL and Bearer token. Prefers client_credentials if configured."""
    base_url = os.environ.get("COMP_AI_BASE_URL") or _get_variable("COMP_AI_BASE_URL")
    if not base_url:
        raise ValueError("COMP_AI_BASE_URL must be set (Airflow Variable or env)")
    base_url = base_url.rstrip("/")
    base_url = _ascii_safe(base_url)

    token = _fetch_token_client_credentials()
    if not token:
        token = os.environ.get("COMP_AI_TOKEN") or _get_variable("COMP_AI_TOKEN")
        if not token:
            raise ValueError(
                "COMP_AI_TOKEN must be set, or set COMP_AI_KEYCLOAK_URL, COMP_AI_CLIENT_ID, COMP_AI_CLIENT_SECRET (Airflow Variables or env)"
            )
        token = _ascii_safe(token)
    return base_url, token


def _headers(token: str) -> dict[str, str]:
    return {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json",
    }


def get_tests(control_id: Optional[int] = None) -> list[dict[str, Any]]:
    """GET /api/v1/tests. Returns list of control tests (id, control_id, name, test_type, schedule, ...)."""
    base_url, token = _get_config()
    url = f"{base_url}/api/v1/tests"
    params = {}
    if control_id is not None:
        params["control_id"] = control_id
    resp = requests.get(url, headers=_headers(token), params=params or None, timeout=30)
    resp.raise_for_status()
    return resp.json()


def get_gaps() -> list[dict[str, Any]]:
    """GET /api/v1/controls/gaps. Returns list of controls with no evidence (gaps)."""
    base_url, token = _get_config()
    url = f"{base_url}/api/v1/controls/gaps"
    resp = requests.get(url, headers=_headers(token), timeout=30)
    resp.raise_for_status()
    return resp.json()


def post_test_result(
    base_url: str,
    token: str,
    control_id: int,
    test_id: int,
    result: str,
    details: Optional[str] = None,
) -> dict[str, Any]:
    """POST /api/v1/controls/:id/tests/:test_id/result. result: pass | fail | skipped."""
    url = f"{base_url}/api/v1/controls/{control_id}/tests/{test_id}/result"
    body = {"result": result, "details": details}
    resp = requests.post(url, headers=_headers(token), json=body, timeout=30)
    resp.raise_for_status()
    return resp.json()


def run_test_and_record(test: dict[str, Any], base_url: str, token: str) -> dict[str, Any]:
    """
    Run a single control test (stub: always pass with details) and record result via API.
    Later: dispatch by test_type / name to real checks (e.g. GitHub, IdP).
    """
    test_id = test["id"]
    control_id = test["control_id"]
    name = test.get("name", "")
    test_type = test.get("test_type", "manual")

    # Stub: no real check yet; return pass with placeholder details.
    # Future: if test_type == "integration" and name mentions "GitHub", call GitHub API; etc.
    result = "pass"
    details = f"Scheduled run (Airflow); test_type={test_type}"

    return post_test_result(base_url, token, control_id, test_id, result, details)
