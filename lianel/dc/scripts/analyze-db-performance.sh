#!/bin/bash
# Database Performance Analysis Script
# Analyzes slow queries, missing indexes, and table statistics

set -e

DB_HOST="${POSTGRES_HOST:-172.18.0.1}"
DB_PORT="${POSTGRES_PORT:-5432}"
DB_USER="${POSTGRES_USER:-postgres}"
DB_NAME="lianel_energy"

echo "=== Database Performance Analysis ==="
echo "Database: ${DB_NAME}@${DB_HOST}:${DB_PORT}"
echo ""

# Check if psql is available
if ! command -v psql &> /dev/null; then
    echo "Error: psql not found. Install PostgreSQL client tools."
    exit 1
fi

# Function to run SQL query
run_query() {
    local query="$1"
    local description="$2"
    
    echo "## ${description}"
    PGPASSWORD="${POSTGRES_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d "${DB_NAME}" -c "${query}" 2>&1 || echo "Query failed"
    echo ""
}

# 1. Table Sizes
echo "=== Table Sizes ==="
run_query "
SELECT 
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size,
    pg_total_relation_size(schemaname||'.'||tablename) AS size_bytes
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
" "Table Sizes"

# 2. Index Usage
echo "=== Index Usage Statistics ==="
run_query "
SELECT 
    schemaname,
    tablename,
    indexname,
    idx_scan as index_scans,
    idx_tup_read as tuples_read,
    idx_tup_fetch as tuples_fetched
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
ORDER BY idx_scan ASC;
" "Index Usage (unused indexes at top)"

# 3. Missing Indexes (tables with sequential scans)
echo "=== Tables with Sequential Scans ==="
run_query "
SELECT 
    schemaname,
    relname as table_name,
    seq_scan,
    seq_tup_read,
    idx_scan,
    seq_tup_read / seq_scan as avg_seq_read
FROM pg_stat_user_tables
WHERE schemaname = 'public'
  AND seq_scan > 0
ORDER BY seq_tup_read DESC
LIMIT 20;
" "Tables with Sequential Scans"

# 4. Slow Queries (if pg_stat_statements is enabled)
echo "=== Slow Queries (if pg_stat_statements enabled) ==="
run_query "
SELECT 
    query,
    calls,
    total_exec_time,
    mean_exec_time,
    max_exec_time
FROM pg_stat_statements
WHERE query NOT LIKE '%pg_stat%'
ORDER BY mean_exec_time DESC
LIMIT 10;
" "Slow Queries" || echo "pg_stat_statements not enabled"

# 5. Table Statistics
echo "=== Table Statistics ==="
run_query "
SELECT 
    schemaname,
    tablename,
    n_live_tup as row_count,
    n_dead_tup as dead_rows,
    last_vacuum,
    last_autovacuum,
    last_analyze,
    last_autoanalyze
FROM pg_stat_user_tables
WHERE schemaname = 'public'
ORDER BY n_live_tup DESC;
" "Table Statistics"

# 6. Index Recommendations
echo "=== Index Recommendations ==="
echo "Review tables with high sequential scans and consider adding indexes on:"
echo "- Frequently filtered columns"
echo "- JOIN columns"
echo "- ORDER BY columns"
echo "- Foreign key columns"
echo ""

echo "=== Analysis Complete ==="
echo "Review the output above to identify:"
echo "1. Large tables that may need partitioning"
echo "2. Unused indexes that can be dropped"
echo "3. Tables with sequential scans that need indexes"
echo "4. Tables that need VACUUM/ANALYZE"