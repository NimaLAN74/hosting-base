#!/usr/bin/env python3
import argparse
import json
import sys
from urllib.parse import urlencode

import requests


def main() -> int:
    p = argparse.ArgumentParser(description="Validate dynamic hybrid symbol selection quality gates.")
    p.add_argument("--base-url", required=True)
    p.add_argument("--quantile", type=float, default=0.2)
    p.add_argument("--short", default="true")
    p.add_argument("--train-days", type=int, default=120)
    p.add_argument("--top", type=int, default=20)
    p.add_argument("--min-selected", type=int, default=8)
    p.add_argument("--max-missing-price-ratio", type=float, default=0.35)
    p.add_argument("--insecure-ssl", action="store_true")
    args = p.parse_args()

    short_flag = str(args.short).strip().lower() in {"1", "true", "yes", "y", "on"}
    query = urlencode(
        {
            "quantile": args.quantile,
            "short": "true" if short_flag else "false",
            "train_days": args.train_days,
            "top": args.top,
        }
    )
    url = f"{args.base_url.rstrip('/')}/api/v1/stock-service/selection?{query}"

    r = requests.get(url, timeout=60, verify=not args.insecure_ssl)
    if r.status_code != 200:
        print(f"[FAIL] selection endpoint returned {r.status_code}: {r.text[:300]}")
        return 1

    data = r.json()
    selected = data.get("selected") or []
    selected_size = int(data.get("selected_size") or len(selected))
    if selected_size < args.min_selected:
        print(f"[FAIL] selected_size={selected_size} < min_selected={args.min_selected}")
        return 1

    missing_price = sum(1 for row in selected if row.get("price") is None)
    missing_ratio = (missing_price / selected_size) if selected_size else 1.0
    if missing_ratio > args.max_missing_price_ratio:
        print(
            f"[FAIL] missing-price-ratio={missing_ratio:.2f} > max={args.max_missing_price_ratio:.2f}"
        )
        return 1

    under50_count = sum(
        1
        for row in selected
        if row.get("price") is not None and float(row.get("price")) <= float(data.get("price_priority_usd") or 50.0)
    )
    summary = {
        "selected_size": selected_size,
        "under50_count": under50_count,
        "missing_price_ratio": round(missing_ratio, 4),
        "model": data.get("model"),
        "model_version": data.get("model_version"),
        "weights": data.get("weights"),
    }
    print("[PASS] dynamic selection gates passed")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())

