import { authenticatedFetch } from '../keycloak.js';

const buildQuery = (params = {}) => {
  const q = new URLSearchParams();
  Object.entries(params).forEach(([k, v]) => {
    if (v !== undefined && v !== null && String(v).trim() !== '') {
      q.set(k, String(v));
    }
  });
  const qs = q.toString();
  return qs ? `?${qs}` : '';
};

export const compAiApi = {
  async getHealth() {
    // Health endpoint is public, no auth required
    const res = await fetch('/api/v1/comp-ai/health');
    if (!res.ok) throw new Error('Health check failed');
    return res.json();
  },

  async getFrameworks() {
    const res = await fetch('/api/v1/comp-ai/frameworks');
    if (!res.ok) throw new Error('Failed to fetch frameworks');
    return res.json();
  },

  async processRequest(prompt, framework = null, messages = null) {
    try {
      const body = { prompt };
      if (framework && String(framework).trim() !== '') {
        body.framework = String(framework).trim();
      }
      if (messages && Array.isArray(messages) && messages.length > 0) {
        body.messages = messages;
      }
      const res = await authenticatedFetch('/api/v1/comp-ai/process', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(body),
      });
      if (!res.ok) {
        if (res.status === 429) {
          const data = await res.json().catch(() => ({}));
          const secs = data.retry_after_secs || 60;
          throw new Error(`Too many requests. Please try again in ${secs} seconds.`);
        }
        const errorText = await res.text().catch(() => 'Unknown error');
        throw new Error(`Failed to process request: ${res.status} ${res.statusText} - ${errorText}`);
      }
      return res.json();
    } catch (err) {
      if (err.message.includes('Not authenticated') || err.message.includes('Unauthorized')) {
        throw new Error('Authentication required. Please log in again.');
      }
      throw err;
    }
  },

  async getRequestHistory({ limit = 50, offset = 0 } = {}) {
    try {
      const res = await authenticatedFetch(
        `/api/v1/comp-ai/history${buildQuery({ limit, offset })}`
      );
      if (!res.ok) {
        if (res.status === 429) {
          const data = await res.json().catch(() => ({}));
          const secs = data.retry_after_secs || 60;
          throw new Error(`Too many requests. Please try again in ${secs} seconds.`);
        }
        const errorText = await res.text().catch(() => 'Unknown error');
        throw new Error(`Failed to fetch request history: ${res.status} ${res.statusText} - ${errorText}`);
      }
      return res.json();
    } catch (err) {
      if (err.message.includes('Not authenticated') || err.message.includes('Unauthorized')) {
        throw new Error('Authentication required. Please log in again.');
      }
      throw err;
    }
  },

  // Phase 4: Controls & Evidence
  /** GET /api/v1/controls. Optional category (C5): e.g. operational, administrative. */
  async getControls({ category } = {}) {
    const qs = category && String(category).trim() ? `?category=${encodeURIComponent(String(category).trim())}` : '';
    const res = await authenticatedFetch(`/api/v1/controls${qs}`);
    if (!res.ok) throw new Error('Failed to fetch controls');
    return res.json();
  },

  async getControl(id) {
    const res = await authenticatedFetch(`/api/v1/controls/${id}`);
    if (!res.ok) {
      if (res.status === 404) throw new Error('Control not found');
      throw new Error('Failed to fetch control');
    }
    return res.json();
  },

  /** G8: Update control external_id (align with external control sets / frameworks). body: { external_id: string | null } */
  async patchControl(id, body) {
    const res = await authenticatedFetch(`/api/v1/controls/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      if (res.status === 404) throw new Error('Control not found');
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || 'Failed to update control');
    }
    return res.json();
  },

  /** Phase 5: audit export (controls + requirements + evidence). format: undefined = JSON, 'csv' = CSV blob. */
  async getControlsExport(format = undefined) {
    const url = format === 'csv'
      ? '/api/v1/controls/export?format=csv'
      : '/api/v1/controls/export';
    const res = await authenticatedFetch(url);
    if (!res.ok) throw new Error('Failed to fetch audit export');
    if (format === 'csv') return res.blob();
    return res.json();
  },

  /** Phase 5: controls with no evidence (gaps). */
  async getControlsGaps() {
    const res = await authenticatedFetch('/api/v1/controls/gaps');
    if (!res.ok) throw new Error('Failed to fetch gaps');
    return res.json();
  },

  /** GET /api/v1/tests – list control tests (for status dashboard). */
  async getTests() {
    const res = await authenticatedFetch('/api/v1/tests');
    if (!res.ok) throw new Error('Failed to fetch tests');
    return res.json();
  },

  /** G6: control–policy mapping (controls with policy/document evidence). */
  async getControlPolicyMapping() {
    const res = await authenticatedFetch('/api/v1/controls/policy-mapping');
    if (!res.ok) throw new Error('Failed to fetch policy mapping');
    return res.json();
  },

  /** G6: SOC 2 System Description template. */
  async getSystemDescription() {
    const res = await authenticatedFetch('/api/v1/system-description');
    if (!res.ok) throw new Error('Failed to fetch system description');
    return res.json();
  },

  /** Phase 5: remediation tasks (optional filter by control_id). */
  async getRemediation({ control_id } = {}) {
    const res = await authenticatedFetch(
      `/api/v1/remediation${buildQuery({ control_id })}`
    );
    if (!res.ok) throw new Error('Failed to fetch remediation tasks');
    return res.json();
  },

  /** Phase 5: get remediation for one control. Returns task or null (200 with null when no task). */
  async getControlRemediation(controlId) {
    const res = await authenticatedFetch(`/api/v1/controls/${controlId}/remediation`);
    if (!res.ok) throw new Error('Failed to fetch remediation');
    const data = await res.json();
    return data ?? null;
  },

  /** Phase 5: create or update remediation for a control. */
  async putControlRemediation(controlId, body) {
    const res = await authenticatedFetch(`/api/v1/controls/${controlId}/remediation`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || data.detail || 'Failed to save remediation');
    }
    return res.json();
  },

  /** Phase 6C / 7.1: AI remediation suggestion for a control. body: { context?: string }. Returns { suggestion, model_used }. */
  async postRemediationSuggest(controlId, body = {}) {
    const res = await authenticatedFetch(`/api/v1/controls/${controlId}/remediation/suggest`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      if (res.status === 503) throw new Error(data.error || 'AI model is not available. Try again later.');
      throw new Error(data.error || data.detail || 'Failed to get suggestion');
    }
    return res.json();
  },

  /** Phase 7.2: AI gap/risk analysis. body: { framework?: string }. Returns { summary, model_used }. */
  async postAnalysisGaps(body = {}) {
    const res = await authenticatedFetch('/api/v1/analysis/gaps', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      if (res.status === 503) throw new Error(data.error || 'AI model is not available. Try again later.');
      throw new Error(data.error || data.detail || 'Failed to analyse gaps');
    }
    return res.json();
  },

  async getEvidence({ control_id, limit = 50, offset = 0 } = {}) {
    const res = await authenticatedFetch(
      `/api/v1/evidence${buildQuery({ control_id, limit, offset })}`
    );
    if (!res.ok) throw new Error('Failed to fetch evidence');
    return res.json();
  },

  async postEvidence(body) {
    const res = await authenticatedFetch('/api/v1/evidence', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!res.ok) throw new Error('Failed to add evidence');
    return res.json();
  },

  /** Phase B: upload file (PDF or text). Multipart: control_id, type, file. */
  async postEvidenceUpload(controlId, type, file) {
    const form = new FormData();
    form.append('control_id', String(controlId));
    form.append('type', type);
    form.append('file', file);
    const res = await authenticatedFetch('/api/v1/evidence/upload', {
      method: 'POST',
      body: form,
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || 'Upload failed');
    }
    return res.json();
  },

  /** G9: Review all evidence for a control – AI flags gaps and suggests fixes. */
  async postControlEvidenceReview(controlId) {
    const res = await authenticatedFetch(`/api/v1/controls/${controlId}/evidence/review`, { method: 'POST' });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || 'Evidence review failed');
    }
    return res.json();
  },

  /** Phase B: AI analyse document (evidence must have extracted text). */
  async postEvidenceAnalyze(evidenceId) {
    const res = await authenticatedFetch(`/api/v1/evidence/${evidenceId}/analyze`, { method: 'POST' });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || 'Analysis failed');
    }
    return res.json();
  },

  async postGitHubEvidence(body) {
    const res = await authenticatedFetch('/api/v1/integrations/github/evidence', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || data.detail || 'Failed to collect GitHub evidence');
    }
    return res.json();
  },

  /** G3: Collect evidence from Okta IdP. body: { control_id, evidence_type } */
  async postOktaEvidence(body) {
    const res = await authenticatedFetch('/api/v1/integrations/okta/evidence', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || data.detail || 'Failed to collect Okta evidence');
    }
    return res.json();
  },

  /** G4: Collect evidence from AWS IAM. body: { control_id, evidence_type } */
  async postAwsEvidence(body) {
    const res = await authenticatedFetch('/api/v1/integrations/aws/evidence', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || data.detail || 'Failed to collect AWS evidence');
    }
    return res.json();
  },

  /** D3: Collect email metadata from M365 (Microsoft Graph). body: { control_id, limit? }. Returns { created, evidence_ids }. */
  async postM365Evidence(body) {
    const res = await authenticatedFetch('/api/v1/integrations/m365/evidence', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || data.detail || 'Failed to collect M365 email evidence');
    }
    return res.json();
  },

  /** C3: List files in Google Drive folder and create evidence (one per file). body: { control_id, folder_id?, limit? }. */
  async postDriveEvidence(body) {
    const res = await authenticatedFetch('/api/v1/integrations/drive/evidence', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || data.detail || 'Failed to collect Drive evidence');
    }
    return res.json();
  },

  /** SharePoint: List files in site/document library and create evidence (one per file). body: { control_id, site_id, drive_id?, folder_path?, limit? }. */
  async postSharepointEvidence(body) {
    const res = await authenticatedFetch('/api/v1/integrations/sharepoint/evidence', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || data.detail || 'Failed to collect SharePoint evidence');
    }
    return res.json();
  },

  /** D4: Store DLP or compliance scan result as one evidence item. body: { control_id, summary, scan_date?, details?, link_url? }. */
  async postDlpEvidence(body) {
    const res = await authenticatedFetch('/api/v1/integrations/dlp/evidence', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || data.detail || 'Failed to save DLP evidence');
    }
    return res.json();
  },

  /** Phase C1: batch create evidence from document URLs. */
  async postScanDocuments(controlId, documents) {
    const res = await authenticatedFetch('/api/v1/scan/documents', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ control_id: controlId, documents }),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || data.detail || 'Scan failed');
    }
    return res.json();
  },

  /** G8: Bulk set external_id on controls (by internal_id). Body: { updates: [ { internal_id, external_id } ] }. */
  async patchBulkExternalId(updates) {
    const res = await authenticatedFetch('/api/v1/controls/bulk-external-id', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ updates }),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || data.detail || 'Bulk update failed');
    }
    return res.json();
  },

  /** Phase C2: batch upload multiple files; creates evidence for each. */
  async postScanUploadBatch(controlId, type, files) {
    const form = new FormData();
    form.append('control_id', String(controlId));
    form.append('type', type);
    for (let i = 0; i < files.length; i++) {
      form.append('file', files[i]);
    }
    const res = await authenticatedFetch('/api/v1/scan/upload-batch', {
      method: 'POST',
      body: form,
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || data.detail || 'Batch upload failed');
    }
    return res.json();
  },
};
