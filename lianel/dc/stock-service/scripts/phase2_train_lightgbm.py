#!/usr/bin/env python3
"""
Train a LightGBM model on stock-service Phase-2 research rows.

Data source:
  GET {base_url}/api/v1/daily-signals/research?sample_rows=0&train_days=...&test_days=...&short=...

Evaluation:
  Chronological walk-forward proxy using the same long/short quantile + vol20 inverse weighting.
  This is intended as a first baseline to validate Phase-2 scaffolding before adding meta-labeling.
"""

from __future__ import annotations

import argparse
import json
import math
import os
from dataclasses import dataclass
from typing import Iterable

import numpy as np
import pandas as pd


def fetch_json(url: str, timeout_s: int = 60, insecure_ssl: bool = False) -> dict:
    import requests

    if insecure_ssl:
        import urllib3

        urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

    r = requests.get(url, timeout=timeout_s, verify=(not insecure_ssl))
    r.raise_for_status()
    return r.json()


@dataclass(frozen=True)
class EvalResult:
    folds: int
    trading_days: int
    mean_daily: float
    vol_daily: float
    sharpe_252: float
    cumulative_return: float
    max_drawdown: float


def mean(xs: Iterable[float]) -> float:
    xs = list(xs)
    return float(sum(xs) / len(xs)) if xs else 0.0


def std_sample(xs: Iterable[float], m: float) -> float:
    xs = list(xs)
    if len(xs) < 2:
        return 0.0
    v = sum((x - m) ** 2 for x in xs) / (len(xs) - 1)
    return float(math.sqrt(v))


def compute_drawdown(pnls: list[float]) -> float:
    cum = 0.0
    peak = 0.0
    max_dd = 0.0
    for x in pnls:
        cum += x
        peak = max(peak, cum)
        max_dd = min(max_dd, cum - peak)
    return float(max_dd)


def evaluate_long_short_strategy(
    df: pd.DataFrame,
    preds: pd.Series,
    quantile: float,
    short_enabled: bool,
) -> tuple[list[float], int]:
    """
    For each ts, rank by prediction, long top quantile, short bottom quantile.
    Weight by inverse vol20 (feature) within each side.
    """
    daily_pnls: list[float] = []
    unique_ts = np.array(sorted(df["ts"].unique()), dtype=np.int64)
    for ts in unique_ts:
        day = df[df["ts"] == ts].copy()
        day["pred"] = preds.loc[day.index]
        day = day.sort_values("pred", ascending=True)
        n = len(day)
        if n < 2:
            continue
        k = max(1, int(math.ceil(n * quantile)))
        k = min(k, n)

        bottom = day.head(k)
        top = day.tail(k)

        # Inverse-vol weights within side; guard against tiny values.
        eps = 1e-8
        top_w_raw = 1.0 / top["vol20"].clip(lower=eps)
        top_w = top_w_raw / top_w_raw.sum() if top_w_raw.sum() > 0 else (1.0 / len(top))

        pnl = float((top_w * top["y_oc_next"]).sum())

        if short_enabled:
            bot_w_raw = 1.0 / bottom["vol20"].clip(lower=eps)
            bot_w = bot_w_raw / bot_w_raw.sum() if bot_w_raw.sum() > 0 else (1.0 / len(bottom))
            # short leg: -w * y
            pnl += float((-bot_w * bottom["y_oc_next"]).sum())

        daily_pnls.append(pnl)

    return daily_pnls, len(unique_ts)


def chronological_walkforward_eval(
    df: pd.DataFrame,
    quantile: float,
    short_enabled: bool,
    train_days: int,
    test_days: int,
) -> EvalResult:
    unique_ts = np.array(sorted(df["ts"].unique()), dtype=np.int64)
    n_days = len(unique_ts)
    if n_days < (train_days + test_days + 5):
        return EvalResult(
            folds=0,
            trading_days=0,
            mean_daily=0.0,
            vol_daily=0.0,
            sharpe_252=0.0,
            cumulative_return=0.0,
            max_drawdown=0.0,
        )

    # Import here so training can still run if the workflow only imports file.
    import lightgbm as lgb

    daily_pnls_all: list[float] = []
    folds = 0

    # Use step=test_days for non-overlapping test segments.
    start_i = train_days
    step = max(1, test_days)
    while start_i + test_days <= n_days:
        train_ts = unique_ts[start_i - train_days : start_i]
        test_ts = unique_ts[start_i : start_i + test_days]

        train_df = df[df["ts"].isin(train_ts)]
        test_df = df[df["ts"].isin(test_ts)]
        if len(train_df) < 20 or len(test_df) < 2:
            start_i += step
            continue

        X_train = train_df[["mom5", "mom20", "vol20"]]
        y_train = train_df["y_oc_next"]
        X_test = test_df[["mom5", "mom20", "vol20"]]

        model = lgb.LGBMRegressor(
            objective="regression",
            n_estimators=400,
            learning_rate=0.05,
            num_leaves=31,
            subsample=0.8,
            colsample_bytree=0.8,
            random_state=42,
        )
        model.fit(X_train, y_train)

        preds_test = pd.Series(model.predict(X_test), index=test_df.index)

        daily_pnls, _ = evaluate_long_short_strategy(
            test_df,
            preds_test,
            quantile=quantile,
            short_enabled=short_enabled,
        )
        if daily_pnls:
            daily_pnls_all.extend(daily_pnls)
            folds += 1

        start_i += step

    if not daily_pnls_all:
        return EvalResult(
            folds=0,
            trading_days=0,
            mean_daily=0.0,
            vol_daily=0.0,
            sharpe_252=0.0,
            cumulative_return=0.0,
            max_drawdown=0.0,
        )

    m = mean(daily_pnls_all)
    sd = std_sample(daily_pnls_all, m)
    sharpe = (m / sd) * math.sqrt(252.0) if sd > 1e-12 else 0.0
    cum = float(sum(daily_pnls_all))
    max_dd = compute_drawdown(daily_pnls_all)

    return EvalResult(
        folds=folds,
        trading_days=len(daily_pnls_all),
        mean_daily=float(m),
        vol_daily=float(sd),
        sharpe_252=float(sharpe),
        cumulative_return=cum,
        max_drawdown=float(max_dd),
    )


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--base-url", default=os.environ.get("STOCK_SERVICE_BASE_URL", "http://72.60.80.84"))
    ap.add_argument("--quantile", type=float, default=float(os.environ.get("DAILY_SIGNAL_QUANTILE", "0.2")))
    ap.add_argument("--short", dest="short_enabled", default=os.environ.get("DAILY_SIGNAL_SHORT_ENABLED", "true"))
    ap.add_argument("--train-days", type=int, default=int(os.environ.get("DAILY_SIGNAL_TRAIN_DAYS", "120")))
    ap.add_argument("--test-days", type=int, default=int(os.environ.get("DAILY_SIGNAL_TEST_DAYS", "20")))
    ap.add_argument("--sample-rows", type=int, default=int(os.environ.get("DAILY_SIGNAL_SAMPLE_ROWS", "0")))
    ap.add_argument("--timeout-s", type=int, default=int(os.environ.get("STOCK_SERVICE_TIMEOUT_S", "120")))
    ap.add_argument(
        "--insecure-ssl",
        action="store_true",
        default=str(os.environ.get("INSECURE_SSL", "false")).lower() in {"1", "true", "yes"},
    )
    args = ap.parse_args()

    short_enabled = str(args.short_enabled).lower() in {"1", "true", "yes"}
    base_url = args.base_url.rstrip("/")

    url = (
        f"{base_url}/api/v1/daily-signals/research"
        f"?sample_rows={args.sample_rows}"
        f"&train_days={args.train_days}"
        f"&test_days={args.test_days}"
        f"&short={'true' if short_enabled else 'false'}"
        f"&quantile={args.quantile}"
    )
    print(f"Fetching dataset: {url}")
    payload = fetch_json(url, timeout_s=args.timeout_s, insecure_ssl=args.insecure_ssl)

    if not payload.get("data_available", False):
        raise SystemExit(f"Dataset not available: {payload.get('reason')}")

    rows = payload.get("rows_sample", []) or []
    df = pd.DataFrame(rows)
    expected_cols = {"ts", "mom5", "mom20", "vol20", "score", "y_oc_next", "symbol"}
    missing = expected_cols.difference(df.columns)
    if missing:
        raise SystemExit(f"Missing expected columns from response: {sorted(missing)}")

    # Ensure numeric
    for col in ["ts", "mom5", "mom20", "vol20", "y_oc_next"]:
        df[col] = pd.to_numeric(df[col], errors="coerce")
    df = df.replace([np.inf, -np.inf], np.nan).dropna(subset=["ts", "mom5", "mom20", "vol20", "y_oc_next"])

    print(f"Loaded rows: {len(df)} (unique decision-days: {df['ts'].nunique()})")

    res = chronological_walkforward_eval(
        df=df,
        quantile=args.quantile,
        short_enabled=short_enabled,
        train_days=args.train_days,
        test_days=args.test_days,
    )

    print("=== LightGBM Phase-2 baseline (walk-forward) ===")
    print(json.dumps(res.__dict__, indent=2))


if __name__ == "__main__":
    main()

