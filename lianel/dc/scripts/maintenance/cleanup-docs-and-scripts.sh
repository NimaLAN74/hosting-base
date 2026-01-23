#!/bin/bash
# Script to clean up and organize documentation and scripts
# Removes duplicates, moves files to proper locations, removes obsolete files

set -e

REPO_DIR="${1:-$(pwd)}"
cd "$REPO_DIR" || exit 1

echo "=== Documentation and Scripts Cleanup ==="
echo ""

# Files to move from root to docs/fixes (status/fix documents)
ROOT_FIX_FILES=(
    "ALL-FIXES-COMPLETE.md"
    "COMPLETE-TEST-RESULTS.md"
    "CONTAINERS-RESTARTED.md"
    "DATA-QUALITY-DASHBOARD-FIX.md"
    "E2E-TEST-COMPLETE.md"
    "E2E-TEST-RESULTS-FINAL.md"
    "E2E-TEST-RESULTS.md"
    "ELECTRICITY-TIMESERIES-EMPTY-FIX.md"
    "ELECTRICITY-TIMESERIES-FIX.md"
    "ENTSOE-API-401-ERROR.md"
    "ENTSOE-DAG-STATUS.md"
    "ENTSOE-DAG-TRIGGERED.md"
    "FINAL-E2E-TEST-REPORT.md"
    "FINAL-FIXES-STATUS.md"
    "FIXES-PROGRESS.md"
    "FIXES-STATUS-AND-NEXT-STEPS.md"
    "FIXES-SUMMARY.md"
    "FIX-MONITORING-AND-ELECTRICITY.md"
    "GRAFANA-ADMIN-ACCESS-FIX.md"
    "GRAFANA-DATASOURCE-CONFIGURATION-COMPLETE.md"
    "GRAFANA-DATASOURCE-FIX-COMPLETE.md"
    "GRAFANA-DATASOURCE-FIX-DEPLOYED.md"
    "GRAFANA-DATASOURCE-FIXES-COMPLETE.md"
    "GRAFANA-DATASOURCE-FIX-SUMMARY.md"
    "GRAFANA-DATASOURCE-TROUBLESHOOTING-COMPLETE.md"
    "GRAFANA-FIX-COMPLETE.md"
    "GRAFANA-FIX-FINAL-DEPLOYMENT.md"
    "GRAFANA-ROLE-FIX.md"
    "MONITORING-PAGE-FIX-DETAILED.md"
    "MONITORING-PAGE-REDIRECT-FIX.md"
    "MONITORING-PAGE-ROOT-CAUSE-FIX.md"
    "MONITORING-PAGE-TEST-RESULTS.md"
    "MONITORING-REDIRECT-FIX.md"
    "PIPELINE-DEPLOY-SCRIPT-FIX.md"
    "PIPELINE-FIX.md"
    "PIPELINE-MISSING-FILE-DEBUG.md"
    "PRE-ROADMAP-FIXES-COMPLETE-FINAL.md"
    "PRE-ROADMAP-FIXES-COMPLETE.md"
    "PRE-ROADMAP-FIXES-FINAL-STATUS.md"
    "RESTART-CONTAINERS-COMMANDS.md"
    "ROOT-CAUSE-ANALYSIS-AND-FIXES.md"
)

# Files to move to docs/status
ROOT_STATUS_FILES=(
    "ALL-CHANGES-PUSHED.md"
    "DASHBOARD-CLEANUP-COMPLETE.md"
    "DASHBOARDS-IN-REPO-VERIFICATION.md"
    "FRONTEND-REORGANIZATION-COMPLETE.md"
    "FRONTEND-REORGANIZATION-PLAN.md"
)

# Files to keep in root (important reference docs)
KEEP_IN_ROOT=(
    "DAG-FAILURES-ANALYSIS.md"
    "TRIGGER-ENTSOE-DAG.md"
)

echo "1. Moving fix/status files from root to docs/fixes..."
for file in "${ROOT_FIX_FILES[@]}"; do
    if [ -f "$file" ]; then
        # Check if duplicate exists in docs/fixes
        if [ -f "docs/fixes/$file" ]; then
            echo "   ⚠️  Duplicate found: $file (keeping docs/fixes version, removing root)"
            rm -f "$file"
        else
            echo "   📁 Moving: $file -> docs/fixes/"
            mv "$file" "docs/fixes/"
        fi
    fi
done
echo ""

echo "2. Moving status files from root to docs/status..."
for file in "${ROOT_STATUS_FILES[@]}"; do
    if [ -f "$file" ]; then
        if [ -f "docs/status/$file" ]; then
            echo "   ⚠️  Duplicate found: $file (keeping docs/status version, removing root)"
            rm -f "$file"
        else
            echo "   📁 Moving: $file -> docs/status/"
            mv "$file" "docs/status/"
        fi
    fi
done
echo ""

echo "3. Checking for duplicate documentation files..."
# Find duplicates between root and docs subdirectories
find docs -name "*.md" -type f | while read -r doc_file; do
    basename_file=$(basename "$doc_file")
    if [ -f "$basename_file" ] && [[ ! " ${KEEP_IN_ROOT[@]} " =~ " ${basename_file} " ]]; then
        echo "   ⚠️  Duplicate: $basename_file (root and $doc_file)"
        echo "      Keeping: $doc_file, removing root version"
        rm -f "$basename_file"
    fi
done
echo ""

echo "4. Cleaning up obsolete scripts..."
# Scripts that are likely obsolete (one-off fixes that are no longer needed)
OBSOLETE_SCRIPTS=(
    "scripts/fix-auth-flow-order.py"
    "scripts/fix-auth-flow-via-api.py"
    "scripts/fix-identity-provider-redirector.py"
    "scripts/disable-all-conditionals.py"
    "scripts/disable-conditional-2fa.py"
    "scripts/disable-conditional-auth-checks.py"
    "scripts/make-pkce-optional.py"
    "scripts/remove-pkce-requirement.py"
    "scripts/restore-frontend-client-original.py"
    "scripts/fix-frontend-client-pkce.py"
)

for script in "${OBSOLETE_SCRIPTS[@]}"; do
    if [ -f "$script" ]; then
        echo "   🗑️  Marking as obsolete: $script"
        # Move to archive instead of deleting
        mkdir -p scripts/archive/obsolete
        mv "$script" "scripts/archive/obsolete/" 2>/dev/null || echo "      (already moved or doesn't exist)"
    fi
done
echo ""

echo "5. Organizing scripts by category..."
# Move scripts to appropriate subdirectories if not already organized
SCRIPT_MAPPINGS=(
    "scripts/create-electricity-timeseries-table.sh:scripts/deployment"
    "scripts/create-electricity-table-direct.sh:scripts/deployment"
    "scripts/fix-electricity-timeseries.sh:scripts/maintenance"
    "scripts/fix-monitoring-and-electricity.sh:scripts/maintenance"
    "scripts/trigger-entsoe-dag.sh:scripts/deployment"
    "scripts/trigger-entsoe-dag-remote.sh:scripts/deployment"
    "scripts/set-entsoe-token.sh:scripts/maintenance"
    "scripts/get-and-set-entsoe-token.sh:scripts/maintenance"
    "scripts/update-keycloak-frontend-client.sh:scripts/keycloak-setup"
    "scripts/fix-grafana-datasource-password.sh:scripts/maintenance"
    "scripts/restart-monitoring-containers.sh:scripts/maintenance"
    "scripts/restart-energy-service.sh:scripts/maintenance"
    "scripts/configure-grafana-keycloak-roles.sh:scripts/keycloak-setup"
    "scripts/fix-grafana-datasources.sh:scripts/maintenance"
    "scripts/configure-grafana-datasources.sh:scripts/maintenance"
)

for mapping in "${SCRIPT_MAPPINGS[@]}"; do
    IFS=':' read -r script target_dir <<< "$mapping"
    if [ -f "$script" ] && [ ! -f "$target_dir/$(basename "$script")" ]; then
        echo "   📁 Moving: $script -> $target_dir/"
        mkdir -p "$target_dir"
        mv "$script" "$target_dir/"
    fi
done
echo ""

echo "6. Creating verification scripts directory..."
mkdir -p scripts/verification
echo "   ✅ Created scripts/verification/ directory"
echo ""

echo "=== Cleanup Complete ==="
echo ""
echo "Summary:"
echo "  - Moved fix/status files to docs/fixes/"
echo "  - Moved status files to docs/status/"
echo "  - Removed duplicate files"
echo "  - Organized scripts by category"
echo "  - Archived obsolete scripts"
echo ""
