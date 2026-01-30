# Comp-AI: Model Usage & Scaling Plan

**Version**: 1.0  
**Date**: January 2026  
**Scope**: Model usage on current remote host and when to scale (KVM4 or separate AI host).

---

## 0. Stable AI (current default – pipeline)

**What the pipeline deploys today:**

- **Default model**: Ollama with **tinyllama**. Compose defaults: `COMP_AI_OLLAMA_URL=http://ollama:11434`, `COMP_AI_OLLAMA_MODEL=tinyllama` (no `.env` needed).
- **Fallback**: If Ollama is configured but the request fails (container down, model not pulled), the API returns a **mock response** instead of 503 (`COMP_AI_OLLAMA_FALLBACK_TO_MOCK=true`). Set to `false` to return 503 when Ollama is unavailable.
- **Pipeline**: Deploy workflow pulls and starts Ollama (`--profile local-model`), runs `ollama pull tinyllama`, then restarts Comp-AI so it uses Ollama by default.

**Summary:** After a successful deploy, Comp-AI uses Ollama/tinyllama when the container and model are available; otherwise it falls back to mock. Override with `COMP_AI_OLLAMA_URL` / `COMP_AI_OLLAMA_MODEL` in `.env` if needed.

---

## 1. Strategy Summary

- **Start**: Single KVM2 VPS; Comp-AI uses a **hosted API** (OpenAI or Azure). No local model on the host.
- **If needed**: Upgrade main host to KVM4, or move AI workload to a separate KVM2/KVM4.
- **Principle**: Only add cost (bigger box or extra host) when you see real performance or delivery issues.

---

## 2. Phase 1: Current Setup (Single KVM2)

### 2.1 Model usage on the remote host

| Component | Where it runs | Model / API |
|-----------|----------------|-------------|
| **Comp-AI service** | Same host (Docker, `lianel-comp-ai-service`) | **Hosted API only** (e.g. OpenAI or Azure OpenAI). `COMP_AI_API_KEY` set; no local model. |
| **Keycloak, Nginx, Frontend, OAuth2, Postgres, etc.** | Same host | No AI; unchanged. |

**Config (current):**

- `COMP_AI_API_KEY`: set to your provider API key (OpenAI or Azure).
- `COMP_AI_MODEL_PATH`: leave unset or ignore — no local model in Phase 1.
- `COMP_AI_MAX_TOKENS`, `COMP_AI_TEMPERATURE`: use as needed for the hosted model.

**Do not** run a local LLM on this KVM2 in Phase 1: 2 vCPU / 8 GB RAM is tight for Keycloak + DB + Nginx + app; adding a model would risk CPU/RAM pressure and slow responses for everyone.

---

## 2.5 Same-host local model experiment (optional)

To **try** hosting a small AI model on the same KVM2 and observe consequences and possibilities:

- The Comp-AI image is built by the pipeline and deployed to the remote host; no local build or run is required.
- All steps below are performed **on the remote host** (or via your deploy process).

1. **On the remote host** (after pipeline has deployed the image and compose):
   - Add to `.env` (or env file used by Comp-AI):
     - `COMP_AI_OLLAMA_URL=http://ollama:11434`
     - `COMP_AI_OLLAMA_MODEL=tinyllama`
   - Start the stack **with the local-model profile** so Ollama runs:
     - `docker compose -f docker-compose.comp-ai.yaml --profile local-model up -d`
   - Pull a small model (one-off, on the remote host):
     - `docker exec lianel-ollama ollama pull tinyllama`
   - Restart comp-ai-service so it picks up the new env (or redeploy).

2. **Behaviour**: Comp-AI service will call Ollama’s `/api/generate` when both `COMP_AI_OLLAMA_URL` and `COMP_AI_OLLAMA_MODEL` are set; otherwise it falls back to mock (or hosted API when that is implemented). If Ollama is configured but the request fails (e.g. container not running, model not pulled), the service **falls back to mock** by default (`COMP_AI_OLLAMA_FALLBACK_TO_MOCK=true`), so the API does not return 503. Set `COMP_AI_OLLAMA_FALLBACK_TO_MOCK=false` to return 503 when Ollama is unavailable.

3. **What to monitor**: Host CPU and RAM (expect spikes during inference), Comp-AI latency (local inference can be slow on 2 vCPU), and impact on Keycloak/frontend. Use this to decide whether to move to KVM4 or a separate AI host.

4. **To turn off**: Unset `COMP_AI_OLLAMA_URL` (and optionally `COMP_AI_OLLAMA_MODEL`) and restart comp-ai-service; stop Ollama with `docker compose -f docker-compose.comp-ai.yaml --profile local-model stop ollama`.

**Suggested small models on 8 GB RAM (CPU-only):** `tinyllama`, `qwen2:0.5b`, or `phi2` (heavier). Pull only one at a time for the experiment.

---

### 2.2 What to monitor

Use existing Prometheus/Grafana (and app logs) to watch:

- **Host**: CPU and RAM usage (sustained high usage or spikes when Comp-AI is used).
- **Comp-AI**: Request count, latency (p50/p95), error rate, time spent in HTTP client calls to the hosted API.
- **User impact**: Slow login (Keycloak), slow page loads (frontend/nginx), or timeouts on Comp-AI requests.

Keep a simple log or spreadsheet: “When we do X, we see Y” (e.g. “5 concurrent Comp-AI requests → CPU 90%”).

---

## 3. When to Consider Upgrading the Main Host (KVM4)

**Trigger:** Performance or delivery issues on the **same** host that runs www, Keycloak, and Comp-AI.

Examples:

- CPU or RAM regularly high (e.g. >80%) when Comp-AI is used, and users notice slowness or errors.
- Keycloak or frontend becomes slow or unstable during Comp-AI usage.
- You want a bit of headroom for growth (more users, more Comp-AI calls) without changing architecture.

**Action:** Upgrade current VPS from KVM2 → KVM4 (+33 kr/month or equivalent). Keep one host; still use **hosted API** for the model. No need to run a local model on KVM4 unless you explicitly plan to (see below).

---

## 4. When to Consider a Separate AI Host (KVM2 or KVM4)

**Trigger:** You want to isolate AI workload from the main site, or you want to **run a local model**.

**Option A – Separate host, still hosted API**

- **Why:** Isolate traffic and CPU usage so Comp-AI calls (and any retries) don’t compete with Keycloak/frontend/DB.
- **How:** Move **only** `comp-ai-service` to a second VPS (KVM2 is enough if it’s just the service + hosted API). Main host keeps Keycloak, Nginx, frontend, Postgres, etc. Nginx on main host proxies `/api/v1/comp-ai/` to the AI host.
- **When:** Main host is consistently under load and you see latency or errors correlated with Comp-AI usage; moving Comp-AI to its own small KVM2 is a low-cost way to isolate.

**Option B – Separate host with local model**

- **Why:** You decide to run a small local model (e.g. for cost, data residency, or experimentation).
- **How:** Dedicated AI host (KVM2 or KVM4, depending on model size). Run comp-ai-service + model runtime (e.g. Ollama, or a small CPU model) on that host. Main host unchanged; Nginx proxies to AI host as above.
- **When:** You have a concrete need for a local model; don’t run it on the main www host to avoid impacting Keycloak and frontend.

---

## 5. Decision Flow

```
Start: KVM2, Comp-AI = hosted API only
         │
         ▼
   Monitor CPU, RAM, latency, errors
         │
    ┌────┴────┐
    │         │
    ▼         ▼
 Issues?   No issues
    │         │
    ▼         └──► Stay on KVM2 + hosted API
 Consider:
    │
    ├─ Main host overloaded (www + Keycloak + Comp-AI)
    │     → Upgrade to KVM4 (same host, still hosted API)
    │
    └─ Want isolation or local model
          → Separate AI host (KVM2 or KVM4)
            - Hosted API only → small KVM2 enough
            - Local model    → KVM2/KVM4 per model needs
```

---

## 6. Checklist for Phase 1 (Current Setup)

- [ ] Comp-AI service uses **hosted API only** (`COMP_AI_API_KEY` set; no local model).
- [ ] Monitoring in place for host CPU/RAM and Comp-AI latency/errors.
- [ ] Document where to check metrics (Grafana dashboard names, log locations).
- [ ] Revisit this plan when you see sustained high load or user-reported slowness; then decide KVM4 vs separate AI host based on section 3 and 4.

---

## 7. References

- Service config: `lianel/dc/comp-ai-service/src/config.rs` (`COMP_AI_API_KEY`, `COMP_AI_MODEL_PATH`).
- Compose: `lianel/dc/docker-compose.comp-ai.yaml`.
- Broader deployment: `COMP-AI-DEPLOYMENT-PLAN.md`.
