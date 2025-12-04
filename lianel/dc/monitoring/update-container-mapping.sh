#!/bin/bash
# Script to generate container ID to name mapping for Grafana

OUTPUT_FILE="/tmp/container-mapping.txt"

echo "# Container ID to Name Mapping" > "$OUTPUT_FILE"
echo "# Generated at: $(date)" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "| Short ID | Full ID | Container Name |" >> "$OUTPUT_FILE"
echo "|----------|---------|----------------|" >> "$OUTPUT_FILE"

docker ps --format "{{.ID}}\t{{.ID}}\t{{.Names}}" | while IFS=$'\t' read -r short_id full_id name; do
    # Get the full container ID
    full_container_id=$(docker inspect --format='{{.Id}}' "$short_id" 2>/dev/null)
    echo "| $short_id | $full_container_id | $name |" >> "$OUTPUT_FILE"
done

echo "" >> "$OUTPUT_FILE"
echo "To match with Grafana metrics, look for the full container ID in the systemd path:" >> "$OUTPUT_FILE"
echo "/system.slice/docker-<FULL_CONTAINER_ID>.scope" >> "$OUTPUT_FILE"

cat "$OUTPUT_FILE"
