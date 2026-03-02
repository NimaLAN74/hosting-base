# Remote server CPU constantly at 100%

## Quick fix (GitHub Actions)

Uses the same SSH secrets as **Fix Keycloak 502** (`REMOTE_HOST`, `REMOTE_USER`, `SSH_PRIVATE_KEY`).

1. **Actions** → **Analyze CPU on Remote** → **Run workflow**.
2. Check **Apply fix** to restart the top CPU containers after analysis.
3. Open the run and check the **Run CPU analysis on remote** step for:
   - Load average and top processes
   - Per-container CPU (Docker)
   - Which process/container is using the most CPU
   - If fix was applied: restarts and a short re-check

## Run from your machine (SSH with key)

From repo root, with `REMOTE_HOST` and your SSH key path:

```bash
# Analysis only
REMOTE_HOST=your-server SSH_KEY=~/.ssh/deploy_key bash lianel/dc/scripts/run-analyze-cpu-remote.sh

# Analysis + restart top CPU containers
REMOTE_HOST=your-server SSH_KEY=~/.ssh/deploy_key bash lianel/dc/scripts/run-analyze-cpu-remote.sh fix
```

Or set in `.env` at repo root:

- `REMOTE_HOST=...`
- `REMOTE_USER=root` (optional)
- `SSH_KEY=~/.ssh/your_key` (optional if using default SSH key)

Then:

```bash
bash lianel/dc/scripts/run-analyze-cpu-remote.sh
bash lianel/dc/scripts/run-analyze-cpu-remote.sh fix   # with fix
```

## What the analysis does

- **On server** (`lianel/dc/scripts/maintenance/analyze-cpu-on-server.sh`):
  - Prints `uptime` (load average).
  - Top 15 processes by CPU (`ps`).
  - Docker container CPU/memory (`docker stats --no-stream`).
  - Identifies the main CPU consumer and, if it’s a container, suggests or runs `docker restart`.
- **With FIX=1**: Restarts containers using >30% CPU, waits 15s, then re-checks load and `docker stats`.

## If CPU stays high

- Check the **Run CPU analysis on remote** log to see which process or container is at the top.
- Restart that container manually: `docker restart <container_name>` (on the server or via SSH).
- For recurring high CPU: add limits in `docker-compose` (e.g. `cpus: "0.5"`) or scale/optimize the service.
