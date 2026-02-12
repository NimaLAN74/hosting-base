# C3: Google Drive integration (list folder → evidence)

This runbook describes how to configure the Google Drive integration that lists files in a folder and creates one evidence item per file (link only, no content storage).

---

## Environment variables

| Variable | Required | Description |
|----------|----------|-------------|
| `GOOGLE_DRIVE_CREDENTIALS_PATH` | Yes | Path to the service account JSON key file (e.g. `/secrets/drive-sa.json`). |
| `GOOGLE_DRIVE_FOLDER_ID` | Default | Default folder ID to list when the API request does not provide `folder_id`. |

The service account must have **Drive read** access. Share the target folder with the service account email (e.g. `comp-ai-drive@project.iam.gserviceaccount.com`) as Viewer.

---

## How to get credentials

1. In **Google Cloud Console** → **APIs & Services** → **Credentials**, create a **Service account**.
2. Create a **JSON key** for that service account and download it.
3. Place the file on the server (or in a secret mount) and set `GOOGLE_DRIVE_CREDENTIALS_PATH` to that path.
4. Enable the **Google Drive API** for the project.
5. Share the Drive folder you want to list with the service account email (e.g. give it “Viewer” access). Copy the folder ID from the folder URL (`https://drive.google.com/drive/folders/FOLDER_ID`) and set `GOOGLE_DRIVE_FOLDER_ID` or pass it per request.

---

## API

- **POST /api/v1/integrations/drive/evidence**  
  Body: `{ "control_id": number, "folder_id": optional string, "limit": optional number (1–100, default 50) }`.  
  Lists files in the folder (from request or `GOOGLE_DRIVE_FOLDER_ID`), creates one evidence row per file (type `document`, source = file name, link = Drive web link). Returns `{ "created": number, "evidence_ids": number[] }`.

---

## UI

In **Comp AI → Controls & Evidence**, after selecting a control, use the **Collect from Google Drive (folder) — C3** form: optionally set Folder ID (overrides env), set max files, then click **Collect Drive evidence**.

---

## Optional: SharePoint

A SharePoint connector (list docs in a site/library, create evidence per item) can be added in the same way: config, integration module calling Microsoft Graph (e.g. `sites/{site-id}/drive/root/children`), request/response models, handler, and UI. Use the same pattern as M365 (D3) and Drive (C3).
