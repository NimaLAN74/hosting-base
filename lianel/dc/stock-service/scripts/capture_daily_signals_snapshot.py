#!/usr/bin/env python3
"""
Capture daily production signals snapshot from stock-service.

Flow:
1) Try model endpoint.
2) If model has no data_available, fallback to phase-1 endpoint.
3) Save raw JSON payload and a small run summary JSON.
4) Print a concise LONG/SHORT summary + diagnostics for workflow logs.
"""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Tuple

import requests


def parse_bool(v: str) -> bool:
    return str(v).strip().lower() in {"1", "true", "yes", "y", "on"}


def fetch_json(url: str, timeout_s: int, insecure_ssl: bool) -> Dict[str, Any]:
    verify = not insecure_ssl
    if insecure_ssl:
        import urllib3

        urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)
    response = requests.get(url, timeout=timeout_s, verify=verify)
    response.raise_for_status()
    return response.json()


def split_signals(signals: List[Dict[str, Any]]) -> Tuple[List[Dict[str, Any]], List[Dict[str, Any]]]:
    longs: List[Dict[str, Any]] = []
    shorts: List[Dict[str, Any]] = []
    for s in signals:
        side = str(s.get("side", "")).upper()
        if side == "LONG":
            longs.append(s)
        elif side == "SHORT":
            shorts.append(s)
    longs.sort(key=lambda x: float(x.get("weight", 0.0)), reverse=True)
    shorts.sort(key=lambda x: float(x.get("weight", 0.0)), reverse=True)
    return longs, shorts


def fmt_signal_row(s: Dict[str, Any]) -> str:
    sym = str(s.get("symbol", ""))
    w = float(s.get("weight", 0.0))
    score = float(s.get("score", 0.0))
    return f"{sym}: w={w:.4f}, score={score:.4f}"


def main() -> None:
    parser = argparse.ArgumentParser(description="Capture model-first daily signals snapshot")
    parser.add_argument("--base-url", required=True, help="Base URL, e.g. http://host or https://host")
    parser.add_argument("--timeout-s", type=int, default=60)
    parser.add_argument("--insecure-ssl", action="store_true")
    parser.add_argument("--output-dir", default="artifacts/daily-signals")
    parser.add_argument("--quantile", type=float, default=0.2)
    parser.add_argument("--short", default="true")
    parser.add_argument("--train-days", type=int, default=120)
    args = parser.parse_args()

    short_enabled = parse_bool(args.short)
    base_url = args.base_url.rstrip("/")
    model_url = (
        f"{base_url}/api/v1/stock-service/daily-signals/model"
        f"?quantile={args.quantile}&short={'true' if short_enabled else 'false'}&train_days={args.train_days}"
    )
    phase1_url = (
        f"{base_url}/api/v1/stock-service/daily-signals"
        f"?quantile={args.quantile}&short={'true' if short_enabled else 'false'}&backtest=true"
    )

    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    out_dir = Path(args.output_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    source = "model"
    fallback_reason = None

    model_payload = fetch_json(model_url, timeout_s=args.timeout_s, insecure_ssl=args.insecure_ssl)
    payload = model_payload

    if not bool(model_payload.get("data_available", False)):
        source = "phase1_fallback"
        fallback_reason = model_payload.get("reason")
        try:
            payload = fetch_json(phase1_url, timeout_s=args.timeout_s, insecure_ssl=args.insecure_ssl)
            payload["model_unavailable"] = True
            payload["model_unavailable_reason"] = fallback_reason
        except Exception as exc:
            payload = model_payload
            payload["fallback_error"] = str(exc)
            source = "model_unavailable_no_fallback"

    raw_path = out_dir / f"daily-signals-{timestamp}.json"
    raw_path.write_text(json.dumps(payload, indent=2, sort_keys=True), encoding="utf-8")

    signals = payload.get("signals") or []
    longs, shorts = split_signals(signals)
    data_available = bool(payload.get("data_available", False))

    summary = {
        "timestamp_utc": timestamp,
        "source": source,
        "data_available": data_available,
        "reason": payload.get("reason"),
        "model_unavailable_reason": fallback_reason,
        "as_of_ts": payload.get("as_of_ts"),
        "symbols_with_history": payload.get("symbols_with_history"),
        "overlapping_days": payload.get("overlapping_days"),
        "signal_count": len(signals),
        "long_count": len(longs),
        "short_count": len(shorts),
        "top_longs": [fmt_signal_row(s) for s in longs[:10]],
        "top_shorts": [fmt_signal_row(s) for s in shorts[:10]],
        "raw_snapshot_file": str(raw_path),
    }

    summary_path = out_dir / f"daily-signals-summary-{timestamp}.json"
    summary_path.write_text(json.dumps(summary, indent=2, sort_keys=True), encoding="utf-8")

    print("=== Daily Production Signals Snapshot ===")
    print(f"source={source}")
    print(f"data_available={data_available}")
    print(f"reason={payload.get('reason')}")
    print(f"model_unavailable_reason={fallback_reason}")
    print(f"signal_count={len(signals)} longs={len(longs)} shorts={len(shorts)}")
    print(f"as_of_ts={payload.get('as_of_ts')}")
    print(f"symbols_with_history={payload.get('symbols_with_history')}")
    print(f"overlapping_days={payload.get('overlapping_days')}")

    if longs:
        print("Top LONGs:")
        for row in summary["top_longs"]:
            print(f"  - {row}")
    if shorts:
        print("Top SHORTs:")
        for row in summary["top_shorts"]:
            print(f"  - {row}")

    print(f"snapshot_file={raw_path}")
    print(f"summary_file={summary_path}")


if __name__ == "__main__":
    main()
