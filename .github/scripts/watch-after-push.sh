#!/usr/bin/env bash
# After pushing to master, watch the "Deploy Comp AI Service to Production" workflow
# and exit with 0 if it succeeds, 1 if it fails.
# Usage: .github/scripts/watch-after-push.sh [workflow_name]
# Default workflow: "Deploy Comp AI Service to Production"

set -e
WORKFLOW="${1:-Deploy Comp AI Service to Production}"
echo "Waiting for latest run of '$WORKFLOW' to start..."
sleep 10
RUN_ID=$(gh run list --workflow "$WORKFLOW" --limit 1 --json databaseId,status --jq '.[0].databaseId')
if [ -z "$RUN_ID" ]; then
  echo "No run found for workflow '$WORKFLOW'"
  exit 2
fi
echo "Watching run $RUN_ID (refreshing every 5s). Ctrl+C to skip."
gh run watch "$RUN_ID" --exit-status
echo "Run $RUN_ID finished successfully."
