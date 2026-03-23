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


def pick_top_coefficients(coefficients: Dict[str, Any], limit: int = 4) -> List[Tuple[str, float]]:
    pairs: List[Tuple[str, float]] = []
    for k, v in (coefficients or {}).items():
        if k == "intercept":
            continue
        try:
            pairs.append((str(k), float(v)))
        except Exception:
            continue
    pairs.sort(key=lambda kv: abs(kv[1]), reverse=True)
    return pairs[:limit]


def detect_feature_availability(payload: Dict[str, Any]) -> Dict[str, bool]:
    features = payload.get("features") or []
    first = features[0] if features else {}
    return {
        "rank_features": "rank_mom5_cs" in first and "rank_mom20_cs" in first and "rank_vol20_cs" in first,
        "vol_regime": "vol_regime" in first,
    }


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
            payload["model"] = model_payload.get("model")
            payload["training_rows"] = model_payload.get("training_rows")
            payload["coefficients"] = model_payload.get("coefficients")
            payload["model_health"] = model_payload.get("model_health")
            payload["publish_signals"] = model_payload.get("publish_signals", False)
        except Exception as exc:
            payload = model_payload
            payload["fallback_error"] = str(exc)
            source = "model_unavailable_no_fallback"

    raw_path = out_dir / f"daily-signals-{timestamp}.json"
    raw_path.write_text(json.dumps(payload, indent=2, sort_keys=True), encoding="utf-8")

    signals = payload.get("signals") or []
    longs, shorts = split_signals(signals)
    data_available = bool(payload.get("data_available", False))
    model_name = payload.get("model")
    training_rows = payload.get("training_rows")
    coefficients = payload.get("coefficients") or {}
    top_coeffs = pick_top_coefficients(coefficients)
    feature_flags = detect_feature_availability(payload)
    publish_signals = bool(payload.get("publish_signals", data_available))
    publish_reason = payload.get("reason")
    model_health = payload.get("model_health") or {}

    summary = {
        "timestamp_utc": timestamp,
        "source": source,
        "data_available": data_available,
        "reason": payload.get("reason"),
        "publish_signals": publish_signals,
        "publish_reason": publish_reason,
        "model_unavailable_reason": fallback_reason,
        "as_of_ts": payload.get("as_of_ts"),
        "symbols_with_history": payload.get("symbols_with_history"),
        "overlapping_days": payload.get("overlapping_days"),
        "model": model_name,
        "training_rows": training_rows,
        "feature_availability": feature_flags,
        "coefficients": coefficients,
        "model_health": model_health,
        "top_coefficients": [{"name": k, "value": v} for (k, v) in top_coeffs],
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
    print(f"publish_signals={publish_signals}")
    print(f"publish_reason={publish_reason}")
    print(f"model_unavailable_reason={fallback_reason}")
    print(f"signal_count={len(signals)} longs={len(longs)} shorts={len(shorts)}")
    print(f"as_of_ts={payload.get('as_of_ts')}")
    print(f"symbols_with_history={payload.get('symbols_with_history')}")
    print(f"overlapping_days={payload.get('overlapping_days')}")
    print(f"model={model_name}")
    print(f"training_rows={training_rows}")
    print(
        "feature_availability="
        f"rank_features:{feature_flags['rank_features']},vol_regime:{feature_flags['vol_regime']}"
    )
    if model_health:
        print(
            "model_health="
            f"feature_row_count:{model_health.get('feature_row_count')},"
            f"nan_or_inf_count:{model_health.get('nan_or_inf_count')},"
            f"coef_norm:{model_health.get('coef_norm')},"
            f"train_window_days:{model_health.get('train_window_days')}"
        )
    if top_coeffs:
        print("Top coefficients:")
        for name, val in top_coeffs:
            print(f"  - {name}={val:.6f}")

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
