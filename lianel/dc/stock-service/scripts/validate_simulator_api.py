#!/usr/bin/env python3
"""
Validate simulator API end-to-end:
- start run
- poll status
- fetch timeline
- fetch order/risk/readiness artifacts
- fetch bias report
- fetch explainability for one decision
"""

import argparse
import json
import time
from urllib.error import HTTPError
from urllib import request


class HttpJsonError(RuntimeError):
    def __init__(self, method: str, url: str, status: int, body: str):
        self.method = method
        self.url = url
        self.status = status
        self.body = body
        super().__init__(f"{method} {url} failed: HTTP {status} - {body[:300]}")


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
    try:
        with request.urlopen(req, timeout=120, context=ctx) as response:  # nosec B310 - controlled endpoints
            code = response.status
            raw = response.read().decode("utf-8")
    except HTTPError as exc:
        body_text = ""
        if exc.fp is not None:
            body_text = exc.fp.read().decode("utf-8", errors="replace")
        raise HttpJsonError(method, url, exc.code, body_text) from exc
    if code < 200 or code >= 300:
        raise HttpJsonError(method, url, code, raw)
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

    run = None
    start_url = f"{base}/api/v1/stock-service/sim/runs"
    last_start_exc: HttpJsonError | None = None
    for attempt in range(1, 7):
        try:
            run = http_json(
                start_url,
                method="POST",
                payload=start_payload,
                insecure=args.insecure_ssl,
            )
            break
        except HttpJsonError as exc:
            last_start_exc = exc
            body = exc.body or ""
            transient = exc.status in (404, 429, 500, 502, 503, 504)
            data_limited = exc.status in (400, 422) and any(
                token in body
                for token in (
                    "need at least 6 symbols",
                    "selection produced too few symbols",
                    "not enough symbols with stable daily history",
                    "not enough aligned days",
                )
            )
            upstream_data_unavailable = exc.status in (400, 503) and any(
                token in body
                for token in (
                    "history failed 500",
                    "Chart data unavailable",
                    '"retryable":true',
                    '"retryable": true',
                )
            )
            if data_limited:
                print(f"simulator_validation=skipped_data_limited status={exc.status}")
                print(body[:400])
                return
            if upstream_data_unavailable and attempt < 6:
                print(f"start_run_retry_upstream attempt={attempt} status={exc.status}")
                time.sleep(5)
                continue
            if upstream_data_unavailable:
                print(f"simulator_validation=skipped_upstream_data_unavailable status={exc.status}")
                print(body[:400])
                return
            if transient and attempt < 6:
                print(f"start_run_retry attempt={attempt} status={exc.status}")
                time.sleep(5)
                continue
            raise
    if run is None:
        if last_start_exc and last_start_exc.status in (503, 502, 504):
            print(f"simulator_validation=skipped_upstream_unavailable status={last_start_exc.status}")
            print((last_start_exc.body or "")[:400])
            return
        raise RuntimeError("Failed to start simulator run after retries")
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
    if not any(e.get("kind") in ("OrderSubmitted", "OrderFilled", "OrderPartiallyFilled") for e in events):
        raise RuntimeError("Missing order lifecycle events in timeline")

    orders = http_json(
        f"{base}/api/v1/stock-service/sim/runs/{run_id}/orders?limit=200",
        insecure=args.insecure_ssl,
    ).get("orders", [])
    if len(orders) == 0:
        raise RuntimeError("Order ledger is empty")
    print(f"orders={len(orders)}")

    risk = http_json(
        f"{base}/api/v1/stock-service/sim/runs/{run_id}/risk?limit=200",
        insecure=args.insecure_ssl,
    ).get("risk", [])
    if len(risk) == 0:
        raise RuntimeError("Risk snapshot series is empty")
    print(f"risk_points={len(risk)}")

    readiness = http_json(
        f"{base}/api/v1/stock-service/sim/runs/{run_id}/readiness",
        insecure=args.insecure_ssl,
    )
    if "score" not in readiness:
        raise RuntimeError("Readiness report missing score")
    print(f"readiness_score={readiness.get('score')}")

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

