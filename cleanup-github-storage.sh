#!/bin/bash
# GitHub Storage Cleanup Script
# This script helps clean up GitHub Actions artifacts and container images

set -e

echo "🧹 GitHub Storage Cleanup Script"
echo "================================"
echo ""

# Check if GitHub CLI is installed
if ! command -v gh &> /dev/null; then
    echo "❌ GitHub CLI (gh) is not installed"
    echo "Install it from: https://cli.github.com/"
    echo ""
    echo "Or use the manual cleanup steps in GITHUB-STORAGE-CLEANUP.md"
    exit 1
fi

# Check if logged in
if ! gh auth status &> /dev/null; then
    echo "🔐 Not logged in to GitHub CLI"
    echo "Logging in..."
    gh auth login
fi

REPO="NimaLAN74/hosting-base"

echo "📊 Current Storage Usage"
echo "-----------------------"
echo ""

# Check Actions artifacts
echo "📦 Actions Artifacts:"
gh api repos/${REPO}/actions/artifacts --jq '.artifacts | length' || echo "0"
echo ""

# Check Container Registry packages
echo "🐳 Container Registry Packages:"
gh api user/packages?package_type=container --jq '.[] | select(.name | contains("hosting-base")) | {name: .name, size: .size}' || echo "None found"
echo ""

# Ask for confirmation
read -p "Do you want to proceed with cleanup? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cleanup cancelled"
    exit 0
fi

echo ""
echo "🧹 Starting Cleanup..."
echo ""

# Option 1: Delete old artifacts (older than 7 days)
echo "1️⃣  Deleting old artifacts (older than 7 days)..."
ARTIFACTS=$(gh api repos/${REPO}/actions/artifacts --jq '.artifacts[] | select(.created_at < (now - 604800 | strftime("%Y-%m-%d"))) | .id')
if [ -z "$ARTIFACTS" ]; then
    echo "   ✅ No old artifacts to delete"
else
    echo "$ARTIFACTS" | while read -r id; do
        echo "   Deleting artifact ID: $id"
        gh api -X DELETE repos/${REPO}/actions/artifacts/$id || true
    done
    echo "   ✅ Old artifacts deleted"
fi
echo ""

# Option 2: Delete old container image versions (keep only latest)
echo "2️⃣  Cleaning up container images..."
echo "   ⚠️  This requires manual review"
echo "   Go to: https://github.com/${REPO}/pkgs"
echo "   Delete old versions manually, keeping only 'latest'"
echo ""

# Option 3: Delete old workflow runs (older than 30 days)
echo "3️⃣  Deleting old workflow runs (older than 30 days)..."
WORKFLOWS=("deploy-frontend.yml" "deploy-profile-service.yml")
for workflow in "${WORKFLOWS[@]}"; do
    echo "   Processing workflow: $workflow"
    RUNS=$(gh run list --workflow="$workflow" --limit 100 --json databaseId,createdAt --jq '.[] | select(.createdAt < (now - 2592000 | strftime("%Y-%m-%d"))) | .databaseId')
    if [ -z "$RUNS" ]; then
        echo "   ✅ No old runs to delete for $workflow"
    else
        echo "$RUNS" | while read -r id; do
            echo "   Deleting run ID: $id"
            gh run delete "$id" --confirm || true
        done
        echo "   ✅ Old runs deleted for $workflow"
    fi
done
echo ""

echo "✅ Cleanup Complete!"
echo ""
echo "📊 Summary:"
echo "- Old artifacts deleted"
echo "- Old workflow runs deleted"
echo "- Container images: Please review manually at https://github.com/${REPO}/pkgs"
echo ""
echo "💡 Tip: Update workflows to reduce storage usage:"
echo "   1. Remove artifact upload steps"
echo "   2. Reduce image tags to only 'latest'"
echo "   3. Use direct registry pull instead of artifacts"
echo ""
echo "See GITHUB-STORAGE-CLEANUP.md for details"

