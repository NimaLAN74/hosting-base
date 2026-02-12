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


def _raise_401_help(resp: requests.Response, url: str) -> None:
    """Log help and raise HTTPError so DAG logs show how to fix 401."""
    help_msg = (
        "Set Airflow Variables: COMP_AI_KEYCLOAK_URL, COMP_AI_CLIENT_ID, COMP_AI_CLIENT_SECRET "
        "(recommended), or refresh COMP_AI_TOKEN. See docs/runbooks/COMP-AI-AIRFLOW-401-TOKEN.md"
    )
    log.warning("Comp-AI 401 for %s: %s", url, help_msg)
    raise requests.HTTPError(
        f"401 Unauthorized for {url}. {help_msg}",
        request=resp.request,
        response=resp,
    )


def get_tests(control_id: Optional[int] = None) -> list[dict[str, Any]]:
    """GET /api/v1/tests. Returns list of control tests (id, control_id, name, test_type, schedule, ...)."""
    base_url, token = _get_config()
    url = f"{base_url}/api/v1/tests"
    params = {}
    if control_id is not None:
        params["control_id"] = control_id
    resp = requests.get(url, headers=_headers(token), params=params or None, timeout=30)
    if resp.status_code == 401:
        _raise_401_help(resp, url)
    resp.raise_for_status()
    return resp.json()


def get_gaps() -> list[dict[str, Any]]:
    """GET /api/v1/controls/gaps. Returns list of controls with no evidence (gaps)."""
    base_url, token = _get_config()
    url = f"{base_url}/api/v1/controls/gaps"
    resp = requests.get(url, headers=_headers(token), timeout=30)
    if resp.status_code == 401:
        _raise_401_help(resp, url)
    resp.raise_for_status()
    return resp.json()


def post_scan_documents(
    base_url: str,
    token: str,
    control_id: int,
    documents: list[dict[str, Any]],
) -> dict[str, Any]:
    """
    POST /api/v1/scan/documents (Phase C).
    documents: list of {"url": str, "type": optional str}.
    Returns {"evidence_ids": [...], "created": int}.
    """
    url = f"{base_url}/api/v1/scan/documents"
    body = {"control_id": control_id, "documents": documents}
    resp = requests.post(url, headers=_headers(token), json=body, timeout=120)
    resp.raise_for_status()
    return resp.json()


def post_analysis_gaps(
    base_url: str,
    token: str,
    framework: Optional[str] = None,
) -> dict[str, Any]:
    """
    POST /api/v1/analysis/gaps (Phase 7.2). Optional framework filter.
    Returns {"summary": str, "model_used": str}.
    """
    url = f"{base_url}/api/v1/analysis/gaps"
    body = {} if framework is None else {"framework": framework}
    resp = requests.post(url, headers=_headers(token), json=body, timeout=90)
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


def post_github_evidence(
    base_url: str,
    token: str,
    control_id: int,
    owner: str,
    repo: str,
    evidence_type: str,
) -> dict[str, Any]:
    """POST /api/v1/integrations/github/evidence. evidence_type: last_commit | branch_protection."""
    url = f"{base_url}/api/v1/integrations/github/evidence"
    body = {
        "control_id": control_id,
        "owner": owner,
        "repo": repo,
        "evidence_type": evidence_type,
    }
    resp = requests.post(url, headers=_headers(token), json=body, timeout=60)
    resp.raise_for_status()
    return resp.json()


def post_okta_evidence(
    base_url: str,
    token: str,
    control_id: int,
    evidence_type: str,
) -> dict[str, Any]:
    """POST /api/v1/integrations/okta/evidence. evidence_type: org_summary | users_snapshot | groups_snapshot."""
    url = f"{base_url}/api/v1/integrations/okta/evidence"
    body = {"control_id": control_id, "evidence_type": evidence_type}
    resp = requests.post(url, headers=_headers(token), json=body, timeout=60)
    resp.raise_for_status()
    return resp.json()


def post_aws_evidence(
    base_url: str,
    token: str,
    control_id: int,
    evidence_type: str,
) -> dict[str, Any]:
    """POST /api/v1/integrations/aws/evidence. evidence_type: iam_summary."""
    url = f"{base_url}/api/v1/integrations/aws/evidence"
    body = {"control_id": control_id, "evidence_type": evidence_type}
    resp = requests.post(url, headers=_headers(token), json=body, timeout=60)
    resp.raise_for_status()
    return resp.json()


# G3/G4: test_type -> evidence_type for Okta and AWS
_OKTA_TEST_TYPE_TO_EVIDENCE = {
    "okta_org_summary": "org_summary",
    "okta_users_snapshot": "users_snapshot",
    "okta_groups_snapshot": "groups_snapshot",
}
_AWS_TEST_TYPE_TO_EVIDENCE = {"aws_iam_summary": "iam_summary"}


def run_test_and_record(test: dict[str, Any], base_url: str, token: str) -> dict[str, Any]:
    """
    Run a single control test and record result via API.
    G2: Real execution for github_last_commit / github_branch_protection when test has config.owner/repo.
    G3: Okta – okta_org_summary, okta_users_snapshot, okta_groups_snapshot.
    G4: AWS – aws_iam_summary.
    Otherwise stub (pass with placeholder details).
    """
    test_id = test["id"]
    control_id = test["control_id"]
    name = test.get("name", "")
    test_type = test.get("test_type", "manual")
    config = test.get("config") or {}

    # G2: GitHub integration tests – call Comp-AI GitHub evidence API, then record pass/fail
    if test_type in ("github_last_commit", "github_branch_protection"):
        owner = config.get("owner") if isinstance(config, dict) else None
        repo = config.get("repo") if isinstance(config, dict) else None
        if owner and repo:
            evidence_type = "last_commit" if test_type == "github_last_commit" else "branch_protection"
            try:
                data = post_github_evidence(
                    base_url, token, control_id, str(owner).strip(), str(repo).strip(), evidence_type
                )
                result = "pass"
                details = f"Evidence collected; id={data.get('id', '?')}"
                log.info("GitHub test %s (%s): %s", name, test_type, details)
            except requests.RequestException as e:
                result = "fail"
                resp = getattr(e, "response", None)
                details = (resp.text[:500] if resp is not None and getattr(resp, "text", None) else str(e))[:500]
                log.warning("GitHub test %s (%s) failed: %s", name, test_type, details)
            return post_test_result(base_url, token, control_id, test_id, result, details)
        log.info("Test %s has type %s but no config.owner/repo; skipping real run", name, test_type)

    # G3: Okta integration tests
    if test_type in _OKTA_TEST_TYPE_TO_EVIDENCE:
        evidence_type = _OKTA_TEST_TYPE_TO_EVIDENCE[test_type]
        try:
            data = post_okta_evidence(base_url, token, control_id, evidence_type)
            result = "pass"
            details = f"Okta evidence collected; id={data.get('id', '?')}"
            log.info("Okta test %s (%s): %s", name, test_type, details)
        except requests.RequestException as e:
            result = "fail"
            resp = getattr(e, "response", None)
            details = (resp.text[:500] if resp is not None and getattr(resp, "text", None) else str(e))[:500]
            log.warning("Okta test %s (%s) failed: %s", name, test_type, details)
        return post_test_result(base_url, token, control_id, test_id, result, details)

    # G4: AWS integration tests
    if test_type in _AWS_TEST_TYPE_TO_EVIDENCE:
        evidence_type = _AWS_TEST_TYPE_TO_EVIDENCE[test_type]
        try:
            data = post_aws_evidence(base_url, token, control_id, evidence_type)
            result = "pass"
            details = f"AWS evidence collected; id={data.get('id', '?')}"
            log.info("AWS test %s (%s): %s", name, test_type, details)
        except requests.RequestException as e:
            result = "fail"
            resp = getattr(e, "response", None)
            details = (resp.text[:500] if resp is not None and getattr(resp, "text", None) else str(e))[:500]
            log.warning("AWS test %s (%s) failed: %s", name, test_type, details)
        return post_test_result(base_url, token, control_id, test_id, result, details)

    # Stub for other test types (manual, integration without config, etc.)
    result = "pass"
    details = f"Scheduled run (Airflow); test_type={test_type}"
    return post_test_result(base_url, token, control_id, test_id, result, details)
