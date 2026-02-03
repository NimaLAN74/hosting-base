#!/bin/bash
# Database Backup Script
# Creates daily backups of the lianel_energy database

set -euo pipefail

cd /root/lianel/dc

# Load environment variables
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
fi

POSTGRES_HOST=${POSTGRES_HOST:-172.18.0.1}
POSTGRES_PORT=${POSTGRES_PORT:-5432}
POSTGRES_USER=${POSTGRES_USER:-postgres}
POSTGRES_DB=${POSTGRES_DB:-lianel_energy}

BACKUP_DIR="/root/backups/database"
mkdir -p ${BACKUP_DIR}

# Keep only last 7 days of backups
find ${BACKUP_DIR} -name "*.sql.gz" -mtime +7 -delete

# Create backup (EU day-first notation; replace slashes for filename safety)
STAMP="$(date +%d-%m-%Y_%H%M%S)"
BACKUP_FILE="${BACKUP_DIR}/lianel_energy_${STAMP}.sql"
PGPASSWORD=${POSTGRES_PASSWORD} pg_dump -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d ${POSTGRES_DB} -F c -f ${BACKUP_FILE}

# Compress
gzip ${BACKUP_FILE}

# Verify backup
if [ -f "${BACKUP_FILE}.gz" ]; then
    BACKUP_SIZE=$(du -h "${BACKUP_FILE}.gz" | cut -f1)
    echo "$(date +'%d/%m/%Y %H:%M:%S'): Backup created successfully: ${BACKUP_FILE}.gz (${BACKUP_SIZE})"
    exit 0
else
    echo "$(date): ERROR: Backup failed!" >&2
    exit 1
fi
