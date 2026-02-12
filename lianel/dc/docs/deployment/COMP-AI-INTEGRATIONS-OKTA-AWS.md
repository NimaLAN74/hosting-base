# Comp AI Integrations Setup: Okta (IdP) and AWS IAM

This runbook describes how to configure and operate the Okta and AWS IAM integrations used for control evidence collection.

---

## Okta (IdP) Integration (G3)

### Environment variables

| Variable | Required | Description |
|----------|----------|-------------|
| `OKTA_DOMAIN` | Yes | Your Okta org domain, e.g. `mycompany.okta.com` (no `https://`). |
| `OKTA_API_TOKEN` | Yes | Okta API token with read access to org, users, and groups. |

Set these in your deployment environment (e.g. `.env` for local dev, or secrets in CI/CD or container runtime).

### How to obtain an Okta API token

1. Log in to the **Okta Admin Console** as an admin.
2. Go to **Security → API** (or **API → Tokens** depending on your Okta version).
3. Create a new **API Token**. Give it a label (e.g. `Comp-AI evidence collection`).
4. Copy the token immediately; it is shown only once. Store it securely (e.g. in a secrets manager).
5. Use this value as `OKTA_API_TOKEN`. The token needs at least read access to:
   - Org (`GET /api/v1/org`)
   - Users (`GET /api/v1/users`)
   - Groups (`GET /api/v1/groups`)

### Evidence types

- **org_summary** – Organization summary from Okta.
- **users_snapshot** – Snapshot of users (count and sample).
- **groups_snapshot** – Snapshot of groups (count and sample).

These can be triggered from the Comp AI UI (“Collect Okta evidence”) or via the Airflow control tests DAG when a test is configured with one of: `okta_org_summary`, `okta_users_snapshot`, `okta_groups_snapshot`.

---

## AWS IAM Integration (G4)

### Environment variables / credentials

The service uses the **AWS default credential chain**. You can provide credentials in either of these ways:

| Method | Notes |
|--------|--------|
| **Environment variables** | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`. Optional: `AWS_REGION` (e.g. `eu-north-1`). |
| **Instance/task role** | When running on AWS (EC2, ECS, Lambda), attach an IAM role with the required permissions; no env vars needed for credentials. |

Set `AWS_REGION` if you do not rely on instance metadata or other defaults.

### IAM permissions required

The role or user used by the Comp AI service must be allowed to call:

- `iam:GetAccountSummary` – used to collect IAM account summary (user/group/role counts, MFA status, etc.).

Create an IAM policy with this permission and attach it to the role or user used at runtime.

### Evidence type

- **iam_summary** – IAM account summary (users, groups, roles, MFA-enabled users, etc.).

Trigger from the Comp AI UI (“Collect AWS evidence”) or via the Airflow control tests DAG with `test_type: aws_iam_summary`.

---

## M365 (Email) Integration (D3)

Import recent email metadata from a Microsoft 365 mailbox as evidence (one row per message: From, Subject, Date, link). No body storage.

### Environment variables

| Variable | Required | Description |
|----------|----------|-------------|
| `M365_TENANT_ID` | Yes | Azure AD tenant ID (GUID or xxx.onmicrosoft.com). |
| `M365_CLIENT_ID` | Yes | App (client) ID of the app registration. |
| `M365_CLIENT_SECRET` | Yes | Client secret of the app registration. |
| `M365_MAILBOX_USER_ID` | Yes | User ID or userPrincipalName of the mailbox to read (e.g. user@domain.com). |

### Azure AD app registration

1. In **Azure Portal** → **Azure Active Directory** → **App registrations** → **New registration**. Name the app (e.g. Comp-AI M365 evidence).
2. Under **Certificates & secrets**, create a **Client secret**; use it as `M365_CLIENT_SECRET`.
3. Under **API permissions**, add **Microsoft Graph** → **Application permissions** → **Mail.Read**. Grant admin consent.
4. Use **Application (client) ID** as `M365_CLIENT_ID` and **Directory (tenant) ID** as `M365_TENANT_ID`.
5. Set `M365_MAILBOX_USER_ID` to the user principal name or object ID of the mailbox.

### API and UI

- **POST /api/v1/integrations/m365/evidence** — body: `{ "control_id": N, "limit": 50 }`. Creates up to `limit` evidence rows (max 100).
- **UI:** Controls → “Collect from M365 (email metadata) — D3”.

---

## Optional follow-up

- **More G4 evidence types**: Additional AWS evidence (e.g. config snapshots, CloudTrail summaries) can be added as new evidence types and handlers.
- **Azure (or other cloud)**: A second cloud integration (e.g. Azure AD / Entra) can be added in the same way: config, integration module, evidence request model, handler, and UI/API + DAG test types.
