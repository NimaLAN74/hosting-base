# Dynamic Hybrid Selector Rollout

This document defines validation and fallback policy for the dynamic hybrid symbol selector.

## Scope

- Universe: dynamic, global liquid symbols (`STOCK_DYNAMIC_UNIVERSE_SYMBOLS` optional override).
- Ranking: daily alpha + intraday overlay + under-$50 price-priority + risk penalty.
- Consumption: selection endpoint, paper-trade run/backfill, order-plan generation.

## Promotion Gates

Promotion to active daily operation requires all of:

1. Selection endpoint returns HTTP 200.
2. `selected_size >= 8`.
3. Missing-price ratio in selected set `<= 35%`.
4. No runtime errors from model computation path.

Automated checks are implemented in:

- `scripts/validate_dynamic_selection.py`
- `.github/workflows/stock-service-universe-refresh.yml`
- `.github/workflows/stock-service-phase2-train.yml` (post-train gate)

## Rollback / Fallback

If gates fail or selector becomes unhealthy:

1. Keep service alive and continue serving watchlist/history/today.
2. Fallback to previous stable universe by setting `STOCK_DYNAMIC_UNIVERSE_SYMBOLS` explicitly.
3. Re-run `stock-service-universe-refresh` workflow and confirm green before removing fallback override.

## Operational Knobs

- `STOCK_DYNAMIC_UNIVERSE_SYMBOLS`: comma-separated manual universe override.
- `STOCK_DYNAMIC_TRADE_UNIVERSE_SIZE`: selected symbol count target (default 20).
- `STOCK_PRICE_PRIORITY_THRESHOLD_USD`: under-price preference threshold (default 50).
- `STOCK_HYBRID_WEIGHT_DAILY` (default 0.60)
- `STOCK_HYBRID_WEIGHT_INTRADAY` (default 0.25)
- `STOCK_HYBRID_WEIGHT_UNDER50` (default 0.20)
- `STOCK_HYBRID_WEIGHT_RISK` (default 0.15)

## Verification Checklist

1. `GET /api/v1/stock-service/selection` returns ranked candidates with explainability fields.
2. UI shows selected symbols and score components.
3. Paper-trade run executes using selected dynamic universe.
4. Universe refresh + train workflows are green.
