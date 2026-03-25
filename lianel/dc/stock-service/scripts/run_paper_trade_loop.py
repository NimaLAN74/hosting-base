#!/usr/bin/env python3
"""
Run the paper-trade loop by calling:
  POST {base_url}/api/v1/stock-service/paper-trade/run?quantile=...&short_enabled=...&train_days=...

Saves:
  - full JSON response
  - small text summary for easy inspection in CI logs
"""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict
from urllib.parse import urljoin

import requests


def fetch_json_post(url: str, timeout_s: int, insecure_ssl: bool) -> Dict[str, Any]:
    verify = not insecure_ssl
    if insecure_ssl:
        import urllib3

        urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

    def post_once(target: str) -> requests.Response:
        # Do NOT auto-follow redirects: requests may change POST->GET on 301/302.
        return requests.post(target, timeout=timeout_s, verify=verify, allow_redirects=False)

    r = post_once(url)
    # Preserve method over redirects (handle both absolute + relative locations).
    for _ in range(5):
        if r.status_code in (301, 302, 303, 307, 308) and r.headers.get("Location"):
            nxt = urljoin(r.url, r.headers["Location"])
            r = post_once(nxt)
            continue
        break

    r.raise_for_status()
    return r.json()


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--base-url", required=True, help="Base URL, e.g. http://host or https://host")
    ap.add_argument("--quantile", type=float, default=0.2)
    ap.add_argument("--short-enabled", type=str, default="true")
    ap.add_argument("--train-days", type=int, default=120)
    ap.add_argument("--timeout-s", type=int, default=90)
    ap.add_argument("--insecure-ssl", action="store_true")
    ap.add_argument("--output-dir", default="artifacts/paper-trade")
    args = ap.parse_args()

    short_enabled = args.short_enabled.strip().lower() in {"1", "true", "yes", "y", "on"}
    base_url = args.base_url.rstrip("/")

    url = (
        f"{base_url}/api/v1/stock-service/paper-trade/run"
        f"?quantile={args.quantile}&short_enabled={'true' if short_enabled else 'false'}&train_days={args.train_days}"
    )

    ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    out_dir = Path(args.output_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    res = fetch_json_post(url, timeout_s=args.timeout_s, insecure_ssl=args.insecure_ssl)

    out_path = out_dir / f"paper-trade-run-{ts}.json"
    out_path.write_text(json.dumps(res, indent=2, sort_keys=True), encoding="utf-8")

    print("=== Paper Trade Loop Run ===")
    print(f"ts_utc={ts}")
    print(f"stored_decision={res.get('stored_decision')}")
    print(f"stored_decision_ts={res.get('stored_decision_ts')}")
    print(f"executed_count={res.get('executed_count')}")
    print(f"pending_after={res.get('pending_after')}")
    print(f"response_file={out_path}")


if __name__ == "__main__":
    main()

