#!/bin/bash
# Run ON THE SERVER to analyze 100% CPU usage.
# Collects: load average, top processes, Docker container CPU, and suggests or applies fixes.
# Usage on server: bash analyze-cpu-on-server.sh [FIX=1]
#   FIX=1: after reporting, restart the top CPU container(s) and re-check.

set -e

FIX="${FIX:-0}"
echo "=== CPU / load (one snapshot) ==="
uptime
echo ""
echo "=== Top 15 processes by CPU (%) ==="
ps aux --sort=-%cpu 2>/dev/null | head -16 || ps -eo pid,pcpu,pmem,comm --sort=-pcpu 2>/dev/null | head -16
echo ""
echo "=== Docker container CPU/memory (--no-stream, timeout 25s) ==="
if command -v docker >/dev/null 2>&1; then
  timeout 25 docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.MemPerc}}" 2>/dev/null || echo "(docker stats timed out or failed)"
else
  echo "Docker not available"
fi
echo ""
echo "=== Processes using > 50% CPU (if any) ==="
ps -eo pid,pcpu,pmem,comm --sort=-pcpu 2>/dev/null | awk 'NR==1 || $2+0>50 {print}'
echo ""

# Identify top consumer
TOP_PID=""
TOP_CPU=""
TOP_NAME=""
if command -v ps >/dev/null 2>&1; then
  LINE=$(ps -eo pid,pcpu,comm --sort=-pcpu --no-headers 2>/dev/null | head -1)
  if [ -n "$LINE" ]; then
    TOP_PID=$(echo "$LINE" | awk '{print $1}')
    TOP_CPU=$(echo "$LINE" | awk '{print $2+0}')
    TOP_NAME=$(echo "$LINE" | awk '{for(i=3;i<=NF;i++) printf "%s ", $i; print ""}' | sed 's/ $//')
  fi
fi

echo "=== Summary ==="
if [ -n "$TOP_PID" ] && [ "${TOP_CPU:-0}" -gt 80 ]; then
  echo "Primary CPU consumer: PID $TOP_PID ($TOP_NAME) at ~${TOP_CPU}%"
  # Check if it's a container process (find container whose top includes this PID)
  if command -v docker >/dev/null 2>&1; then
    CONTAINER=""
    for c in $(docker ps -q 2>/dev/null); do
      if docker top "$c" -o pid 2>/dev/null | grep -q "^ *$TOP_PID$"; then
        CONTAINER="$c"
        break
      fi
    done
    if [ -n "$CONTAINER" ]; then
      CNAME=$(docker inspect --format '{{.Name}}' "$CONTAINER" 2>/dev/null | sed 's/^\///')
      echo "  -> Runs inside container: $CNAME"
      if [ "$FIX" = "1" ]; then
        echo "  -> Restarting container $CNAME..."
        docker restart "$CONTAINER" 2>/dev/null && echo "  -> Restarted." || echo "  -> Restart failed."
      else
        echo "  -> To restart this container run: docker restart $CONTAINER (or FIX=1 $0)"
      fi
    else
      echo "  -> Not a Docker process. Consider: kill -9 $TOP_PID (only if safe) or limit with cpulimit."
      if [ "$FIX" = "1" ]; then
        echo "  -> FIX=1 only restarts Docker containers; not killing PID $TOP_PID."
      fi
    fi
  fi
else
  echo "No single process above 80% CPU in this snapshot (load may be from many processes or brief spike)."
fi

# If FIX=1, restart top 3 containers by CPU and re-check
if [ "$FIX" = "1" ] && command -v docker >/dev/null 2>&1; then
  echo ""
  echo "=== Restarting top 3 containers by CPU usage ==="
  timeout 20 docker stats --no-stream --format "{{.Name}}\t{{.CPUPerc}}" 2>/dev/null | while read -r name cpu; do
    cpu_num=$(echo "$cpu" | tr -d '%' | cut -d. -f1)
    if [ -n "$cpu_num" ] && [ "$cpu_num" -gt 30 ] 2>/dev/null; then
      echo "Restarting $name (was $cpu)..."
      docker restart "$name" 2>/dev/null && echo "  OK" || echo "  Failed"
    fi
  done
  echo ""
  echo "Waiting 15s then re-checking CPU..."
  sleep 15
  echo "=== Load after fix ==="
  uptime
  timeout 20 docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}" 2>/dev/null || true
fi

echo ""
echo "Done."
