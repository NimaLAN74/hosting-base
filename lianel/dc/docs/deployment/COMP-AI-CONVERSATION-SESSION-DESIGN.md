# Comp-AI: Prompt Session / Conversation on UI

**Goal:** Let users follow a **conversation** with Comp-AI (multi-turn thread) instead of isolated single prompts.

---

## Should we?

**Yes.** A conversation thread on the UI would:

- Make it easier to drill down (“What about CC6.2?”, “Give an example for our SaaS”) without re-pasting context.
- Keep framework and topic in one place per “session.”
- Improve UX for compliance Q&A (follow-up questions are normal).

---

## Could we?

**Yes.** Two main options:

| Approach | UI | Backend | Effort |
|----------|----|---------|--------|
| **A) Conversation thread only (client-side)** | Show thread of user messages + assistant replies; “New conversation” clears it. | No change. Each request still sends only the **current** prompt. | Low. No context across turns. |
| **B) Conversation with context (recommended)** | Same thread UX. Send **full conversation** with each new user message. | Accept optional `messages` (or `conversation_history`); when present, call Ollama **/api/chat** instead of /api/generate. | Medium. Real multi-turn context. |
| **C) Server-side sessions** | Same thread UX; backend stores session by ID. | New session resource, store turns in DB or cache, send session_id + new message. | Higher. More infra and state. |

**Recommendation: B.** No server-side session storage; UI keeps the thread and sends the full message list with each request. Backend stays stateless and uses [Ollama’s chat API](https://github.com/ollama/ollama/blob/main/docs/api.md#generate-a-chat-message) when history is provided.

---

## Recommended design (B)

### UI

- **Conversation thread:** List of pairs (user message, assistant response). New user message appends to the thread; new assistant response appended after it.
- **Input:** Same as now (prompt textarea + framework selector). Framework is fixed for the whole thread (changing it could start a new conversation or warn).
- **“New conversation”** button: Clears the thread and (optionally) framework; next message starts a fresh thread.
- **Persistence:** Thread in React state only (lost on refresh). Optional later: persist thread in `localStorage` by session id so refresh keeps the thread.

### API

- **Existing:** `POST /api/v1/process` with `{ "prompt": "...", "framework": "soc2" }` → unchanged for single-turn; backend keeps using `/api/generate` when no history.
- **Extended:** Same endpoint accepts optional **conversation history**:
  - `messages: [{ "role": "user" | "assistant", "content": "..." }, ...]`
  - Order: chronological. Last item should be the new user message (or backend appends current `prompt` as last user message).
- **Backend logic:**
  - If `messages` is absent or empty: current behavior (single `prompt` → Ollama `/api/generate`).
  - If `messages` is present: build Ollama chat payload: e.g. system message = framework prefix (if any); then `messages`; call Ollama `POST /api/chat` with `{ "model", "messages", "stream": false }`. Return same response shape as today.

### Ollama

- **Current:** `/api/generate` with a single `prompt` string.
- **With history:** `/api/chat` with `messages: [{ role, content }, ...]`. Same model, no new infra.

### Stored history

- **Request history (DB):** Today we store one row per “request” (one user prompt + one response). For conversations we can either:
  - Keep one row per user message (each row has prompt + response; no explicit session id), or
  - Add optional `session_id` / `conversation_id` and store each turn with that id for “conversation history” later. Prefer starting without session_id and keep one row per turn; add session_id later if we want “list my conversations.”

---

## Implementation order

1. **Backend:** Add optional `messages: Vec<{ role, content }>` to request; when present, call Ollama `/api/chat` and return same response shape; when absent, keep current `/api/generate` behavior.
2. **Frontend:** Add conversation state (array of { user, assistant }), render thread above input, “New conversation” clears state. On submit, send either only `prompt` (no history) or `prompt` + `messages` (full thread) depending on whether thread is non-empty.
3. **Optional:** Persist thread in `localStorage` keyed by e.g. `comp-ai-session-{date}` or a generated id so refresh keeps the conversation.

---

## In-memory cache / DB and performance

**Yes, an in-memory layer can lift performance.** Two ways to use it:

| Use case | What it does | Performance impact |
|----------|----------------|---------------------|
| **Response cache** | Cache `(hash(messages + framework)) → response` with a short TTL. On cache hit, return stored response without calling Ollama. | Fewer Ollama calls, lower latency on repeated or identical prompts/conversation tails. |
| **Session store** (optional) | Store full conversation by `session_id` in Redis (or similar). Client sends only `session_id` + new message; server loads thread from store, calls Ollama, appends turn, saves. | Smaller request payloads; server holds the thread so client doesn’t resend full history every time. |

### Response cache (recommended first)

- **Key:** Hash of `(framework_id, messages[])` (e.g. SHA-256 or stable JSON hash).
- **Value:** Last response (text, tokens_used, model_used); optionally store with timestamp.
- **TTL:** e.g. 5–10 minutes so cache doesn’t grow unbounded and answers stay “fresh enough.”
- **Where:**
  - **Process-local (e.g. `moka` or `lru` in Rust):** No extra infra, per-instance cache. Good for single replica or when duplicate requests often hit the same instance.
  - **Redis:** Shared cache across Comp-AI instances; better hit rate under load. Use if you already have Redis or run multiple replicas.

**Effect:** Identical or repeated prompts (and same conversation prefix) get a fast cache hit instead of another Ollama call. That reduces latency and Ollama load; conversations that repeat a question (e.g. “same but for ISO 27001”) benefit most.

### Session store in Redis (optional later)

- Store `session_id → [ { role, content }, ... ]` with TTL (e.g. 1–24 hours).
- Client sends `session_id` + new `prompt` (and optionally `framework`). Server loads messages, appends user message, calls Ollama, appends assistant reply, writes back.
- **Benefit:** Smaller requests (no full thread in every POST); same UX. **Cost:** Extra dependency (Redis), eviction policy, and slightly more logic.

Use this if you want smaller payloads or to support “resume conversation on another device” later; otherwise Option B without session store is fine.

### Recommendation

1. **Implement Option B** (conversation thread + `/api/chat`) without any cache first.
2. **Add a process-local response cache** (e.g. `moka::Cache` with TTL and max capacity) keyed by hash of request (framework + messages). Measure hit rate and latency.
3. **If you run multiple replicas or want higher hit rate:** Move response cache to Redis (same key/value/TTL idea).
4. **Session store in Redis:** Only if you need smaller payloads or server-held sessions; not required for good performance with Option B.

---

## Summary

| Question | Answer |
|----------|--------|
| **Should we** add prompt session/conversation on the UI? | **Yes** – better UX for follow-up questions and compliance Q&A. |
| **Could we?** | **Yes** – UI thread + optional `messages` in existing API + Ollama `/api/chat` when history is sent. |
| **Suggested approach** | **B:** Conversation thread in UI; send full conversation with each request; backend uses Ollama chat when `messages` is provided; no server-side session storage. |
| **In-memory DB / cache?** | **Yes, for performance.** Start with a **response cache** (key = hash of framework + messages, TTL ~5–10 min). Process-local (e.g. `moka`) first; Redis if multi-replica or higher hit rate. Optional later: Redis session store so client sends only `session_id` + new message. |

This keeps the current single-prompt flow intact and adds a clear path to multi-turn; an in-memory response cache lifts performance without requiring session storage.

---

## Implementation status (Option B + response cache)

- **Backend:** `POST /api/v1/process` accepts optional `messages: [{ role, content }, ...]`. When present, Ollama `/api/chat` is used; otherwise `/api/generate`. Process-local response cache (moka) keyed by hash of (framework + prompt or messages), TTL and max entries from `COMP_AI_RESPONSE_CACHE_TTL_SECS` (default 300) and `COMP_AI_RESPONSE_CACHE_MAX_ENTRIES` (default 1000); set TTL to 0 to disable.
- **Frontend:** Conversation thread above the form, “New conversation” button, follow-up input; full thread sent as `messages` with each request (backend appends current prompt).
- **Verification:** No local build/run. Push and use the deploy pipeline (e.g. Deploy Comp AI Service to Production), or run on the remote host when needed.
