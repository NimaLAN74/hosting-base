# Comp-AI SharePoint connector

List files in a SharePoint site (document library) and create one evidence item per file (link only). Uses Microsoft Graph with the same app as M365 (D3).

## Config

Reuse M365 app credentials; the app must have **Sites.Read.All** (or Files.Read.All) in addition to Mail.Read for M365 email:

- `M365_TENANT_ID`
- `M365_CLIENT_ID`
- `M365_CLIENT_SECRET`

No separate SharePoint env vars. Site ID and optional drive/folder are provided per request.

## API

- **POST /api/v1/integrations/sharepoint/evidence**
- Body: `{ "control_id": number, "site_id": string, "drive_id"?: string, "folder_path"?: string, "limit"?: number }`
- `site_id`: SharePoint site ID (e.g. from site URL or Graph Explorer: `contoso.sharepoint.com,{site-guid},{web-guid}`).
- `drive_id`: optional; if omitted, uses the site’s default document library.
- `folder_path`: optional path relative to drive root (e.g. `Shared Documents/Policies`).
- `limit`: max items (default 50, max 100).

## UI

Controls → Audit → “Collect from SharePoint (document library)”: enter Site ID, optional Drive ID and folder path, then submit. Creates one evidence row per file with link.

## Getting the site ID

1. In Azure AD app registration, ensure the app has **Application permission** **Sites.Read.All** (or **Files.Read.All**).
2. Use [Graph Explorer](https://developer.microsoft.com/en-us/graph/graph-explorer) or `GET https://graph.microsoft.com/v1.0/sites?search=*` to find the site, or build the site ID from the SharePoint URL (tenant hostname + site and web IDs from the URL path).
