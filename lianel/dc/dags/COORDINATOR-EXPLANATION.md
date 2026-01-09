# Coordinator DAG - Do We Need It?

## What the Coordinator Does

The `eurostat_ingestion_coordinator` DAG:
1. **Waits** for all 4 table DAGs to complete using `ExternalTaskSensor`
2. **Verifies** all tables ingested successfully
3. **Automatically triggers** harmonization DAG

## Do We Need It?

### ✅ **YES, if you want:**
- **Automatic orchestration**: Harmonization runs automatically after all ingestion completes
- **Scheduled runs**: On Sunday 02:00 UTC, everything runs in sequence automatically
- **Error handling**: Verifies all tables succeeded before harmonization
- **Production workflow**: Set-and-forget pipeline

### ❌ **NO, if you:**
- **Manually trigger** DAGs individually
- **Don't need** automatic harmonization trigger
- **Prefer** manual control over when harmonization runs
- **Want** to run harmonization independently

## Current Setup

**With Coordinator (Recommended for Production):**
```
Sunday 02:00 UTC:
├── All 4 table DAGs start (parallel)
├── Coordinator waits for all to complete
└── Coordinator triggers harmonization automatically
```

**Without Coordinator (Manual Control):**
```
1. Manually trigger table DAGs
2. Wait for them to complete
3. Manually trigger harmonization when ready
```

## Recommendation

**Keep the coordinator** because:
1. It's useful for scheduled weekly runs
2. You can still manually trigger harmonization if needed
3. It ensures harmonization only runs after successful ingestion
4. It's already set up and working

**You can disable it** by:
- Pausing the coordinator DAG in Airflow UI
- Or removing it if you prefer manual control

## How to Use

**Automatic (with coordinator):**
- Just trigger the 4 table DAGs
- Coordinator will automatically trigger harmonization when done

**Manual (without coordinator):**
- Trigger table DAGs
- Manually trigger harmonization when ready

Both approaches work! The coordinator is just for convenience.
