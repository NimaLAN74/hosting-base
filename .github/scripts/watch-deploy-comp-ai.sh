#!/usr/bin/env bash
# Monitor GitHub Actions for "Deploy Comp AI Service" after a push.
# Usage: from repo root, after `git push`:
#   bash .github/scripts/watch-deploy-comp-ai.sh
#   bash .github/scripts/watch-deploy-comp-ai.sh --wait   # poll until run completes
#
# Requires: gh CLI (brew install gh / apt install gh) and `gh auth status` OK.

set -e
WORKFLOW="deploy-comp-ai-service.yml"
WAIT=""
while [[ $# -gt 0 ]]; do
  case $1 in
    --wait) WAIT=1; shift ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

if ! command -v gh &>/dev/null; then
  echo "gh CLI not found. Install: https://cli.github.com/"
  exit 1
fi

echo "Listing latest run for workflow: $WORKFLOW"
gh run list --workflow="$WORKFLOW" --limit 3

RUN_ID=$(gh run list --workflow="$WORKFLOW" --limit 1 --json databaseId --jq '.[0].databaseId')
if [[ -z "$RUN_ID" ]]; then
  echo "No runs found."
  exit 0
fi

if [[ -n "$WAIT" ]]; then
  echo "Watching run $RUN_ID (waiting for completion)..."
  if ! gh run watch "$RUN_ID" --exit-status; then
    echo ""
    echo "Run failed. Last failed step logs:"
    gh run view "$RUN_ID" --log-failed 2>/dev/null | tail -80
    exit 1
  fi
  echo "Run succeeded."
  exit 0
fi

# One-shot status
STATUS=$(gh run view "$RUN_ID" --json status,conclusion --jq '"\(.status) \(.conclusion // "n/a")"')
echo ""
echo "Latest run: $RUN_ID — $STATUS"
echo "View: https://github.com/$(gh repo view --json nameWithOwner -q .nameWithOwner)/actions/runs/$RUN_ID"
echo ""
echo "To watch until complete: bash .github/scripts/watch-deploy-comp-ai.sh --wait"

if [[ "$STATUS" == *"failure"* ]] || [[ "$STATUS" == *"cancelled"* ]]; then
  echo ""
  echo "Failed step logs (tail):"
  gh run view "$RUN_ID" --log-failed 2>/dev/null | tail -50
  exit 1
fi
exit 0
