# AGENTS.md

## Cursor Cloud specific instructions

### Overview

Lianel is a multi-service SaaS platform with **4 Rust backend services** and **2 React frontends**, all normally deployed via Docker Compose. For local development, services can be built and run natively.

### System dependencies

- **Rust**: Requires stable toolchain >= 1.85 (edition 2024 support). Default toolchain may need `rustup default stable` if pinned to an older version.
- **Node.js**: v22+ (managed via nvm).
- **OpenSSL dev**: `libssl-dev` and `pkg-config` must be installed for Rust `openssl-sys` crate compilation.

### Services

| Service | Path | Type | Dev port | Run command |
|---|---|---|---|---|
| Main Frontend | `lianel/dc/frontend` | React (CRA) | 3001 | `PORT=3001 npx react-scripts start` |
| Stock Monitoring UI | `lianel/dc/stock-monitoring/frontend` | React (CRA) | 3002 | `PORT=3002 npx react-scripts start` |
| Profile Service | `lianel/dc/profile-service` | Rust (Axum) | 3000 | `cargo run` (needs env vars below) |
| Comp-AI Service | `lianel/dc/comp-ai-service` | Rust (Axum) | 3002 | `cargo run` (needs DB + env vars) |
| Stock Monitoring Backend | `lianel/dc/stock-monitoring/backend` | Rust (Axum) | — | `cargo run` (needs DB + env vars) |
| Energy Service (PAUSED) | `lianel/dc/energy-service` | Rust (Axum) | — | `cargo run` (needs DB + env vars) |

### Running Rust services locally

- **Profile Service** can run standalone (no database) with these env vars: `KEYCLOAK_ADMIN_USER`, `KEYCLOAK_ADMIN_PASSWORD`, `BACKEND_CLIENT_SECRET`. It defaults `KEYCLOAK_URL` to `http://keycloak:8080` — override for local testing.
- **Comp-AI, Stock Monitoring Backend, and Energy Service** require a PostgreSQL database connection. They use `sqlx` with compile-time checked queries. Without a database, use `cargo check` to verify compilation.
- Use `RUST_LOG=info` (or `debug`) for log output.

### Testing

- **Stock Monitoring UI**: `cd lianel/dc/stock-monitoring/frontend && npm test` (or `npm run test:ci` for CI mode). Note: `App.test.js` has a pre-existing failure (text mismatch "EU markets MVP" vs actual "Stock Exchange Monitoring"). `smoke.test.js` passes.
- **Main Frontend**: Has `App.test.js` but no `start` or `test` script in `package.json`. Tests fail due to a pre-existing `keycloak-js` ESM/CJS incompatibility with Jest (`SyntaxError: Unexpected token 'export'`). A `transformIgnorePatterns` config would be needed to fix this.
- **Rust services**: No automated test suites in the repo. Use `cargo check` to verify compilation.

### Building

- `cargo check` for all Rust services (fast compilation check)
- `cargo build --release` for optimized binaries
- `npm run build` for frontend production builds

### Gotchas

- The main frontend `package.json` does not include a `start` script; use `npx react-scripts start` directly.
- The default Rust toolchain may be pinned to 1.83 instead of stable. Run `rustup default stable` to fix edition 2024 errors.
- Environment files: `.env.example` at repo root and `lianel/dc/.env.example` contain sample credentials. Copy to `.env` for local use.
- Docker Compose files are in `lianel/dc/` — there are 9 compose files for different service groups (infra, backend, frontend, monitoring, airflow, etc.).
