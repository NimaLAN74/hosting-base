#!/usr/bin/env python3
"""
Validate simulator API end-to-end:
- start run
- poll status
- fetch timeline
- fetch bias report
- fetch explainability for one decision
"""

import argparse
import json
import time
from urllib import request


def http_json(url: str, method: str = "GET", payload: dict | None = None, insecure: bool = False):
    body = None
    headers = {"Accept": "application/json"}
    if payload is not None:
        body = json.dumps(payload).encode("utf-8")
        headers["Content-Type"] = "application/json"
    req = request.Request(url=url, method=method, data=body, headers=headers)
    ctx = None
    if insecure and url.startswith("https://"):
        import ssl

        ctx = ssl._create_unverified_context()  # nosec B323
    with request.urlopen(req, timeout=120, context=ctx) as response:  # nosec B310 - controlled endpoints
        code = response.status
        raw = response.read().decode("utf-8")
    if code < 200 or code >= 300:
        raise RuntimeError(f"{method} {url} failed: HTTP {code} - {raw[:300]}")
    return json.loads(raw) if raw else {}


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--base-url", required=True)
    p.add_argument("--insecure-ssl", action="store_true")
    p.add_argument("--days", type=int, default=7)
    args = p.parse_args()

    base = args.base_url.rstrip("/")
    start_payload = {
        "days": max(5, min(args.days, 30)),
        "top": 12,
        "quantile": 0.2,
        "short_enabled": True,
        "initial_capital_usd": 100,
        "reinvest_profit": True,
        "replay_delay_ms": 0,
    }

    run = http_json(
        f"{base}/api/v1/stock-service/sim/runs",
        method="POST",
        payload=start_payload,
        insecure=args.insecure_ssl,
    )
    run_id = run.get("run_id")
    if not run_id:
        raise RuntimeError(f"Missing run_id in response: {run}")
    print(f"Started run: {run_id}")

    status = None
    for _ in range(20):
        status = http_json(
            f"{base}/api/v1/stock-service/sim/runs/{run_id}/status",
            insecure=args.insecure_ssl,
        )
        print(f"status={status.get('status')}")
        if status.get("status") in ("completed", "failed"):
            break
        time.sleep(1.5)
    if not status:
        raise RuntimeError("No status response")

    timeline = http_json(
        f"{base}/api/v1/stock-service/sim/runs/{run_id}/timeline?limit=300",
        insecure=args.insecure_ssl,
    )
    events = timeline.get("events", [])
    if not events:
        raise RuntimeError("Timeline is empty")
    print(f"timeline_events={len(events)}")

    bias = http_json(
        f"{base}/api/v1/stock-service/sim/runs/{run_id}/bias-report",
        insecure=args.insecure_ssl,
    )
    findings = bias.get("findings", [])
    print(f"bias_findings={len(findings)}")

    decision_events = [e for e in events if e.get("kind") == "DecisionCreated" and (e.get("payload") or {}).get("decision_id")]
    if decision_events:
        decision_id = decision_events[0]["payload"]["decision_id"]
        explain = http_json(
            f"{base}/api/v1/stock-service/sim/runs/{run_id}/decision/{decision_id}/explain",
            insecure=args.insecure_ssl,
        )
        if not explain.get("decision"):
            raise RuntimeError("Explain endpoint missing decision payload")
        print("explainability_ok=true")
    else:
        print("explainability_ok=skipped_no_decision")

    print("simulator_validation=ok")


if __name__ == "__main__":
    main()

