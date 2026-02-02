import React, { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { compAiApi } from './compAiApi';
import PageTemplate from '../PageTemplate';
import './CompAI.css';

function CompAIControls() {
  const [controls, setControls] = useState([]);
  const [selectedControl, setSelectedControl] = useState(null);
  const [controlDetail, setControlDetail] = useState(null);
  const [evidence, setEvidence] = useState([]);
  const [loading, setLoading] = useState(true);
  const [detailLoading, setDetailLoading] = useState(false);
  const [error, setError] = useState(null);
  const [success, setSuccess] = useState(null);

  // Manual evidence form
  const [manualType, setManualType] = useState('');
  const [manualSource, setManualSource] = useState('');
  const [manualDescription, setManualDescription] = useState('');
  const [manualLinkUrl, setManualLinkUrl] = useState('');
  const [manualSubmitting, setManualSubmitting] = useState(false);

  // GitHub evidence form
  const [ghOwner, setGhOwner] = useState('');
  const [ghRepo, setGhRepo] = useState('');
  const [ghEvidenceType, setGhEvidenceType] = useState('last_commit');
  const [ghSubmitting, setGhSubmitting] = useState(false);

  useEffect(() => {
    loadControls();
  }, []);

  useEffect(() => {
    if (selectedControl) {
      loadControlDetail(selectedControl.id);
      loadEvidence(selectedControl.id);
    } else {
      setControlDetail(null);
      setEvidence([]);
    }
  }, [selectedControl]);

  const loadControls = async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await compAiApi.getControls();
      setControls(Array.isArray(list) ? list : []);
    } catch (err) {
      setError(err.message || 'Failed to load controls');
    } finally {
      setLoading(false);
    }
  };

  const loadControlDetail = async (id) => {
    setDetailLoading(true);
    try {
      const detail = await compAiApi.getControl(id);
      setControlDetail(detail);
    } catch {
      setControlDetail(null);
    } finally {
      setDetailLoading(false);
    }
  };

  const loadEvidence = async (controlId) => {
    try {
      const list = await compAiApi.getEvidence({ control_id: controlId });
      setEvidence(Array.isArray(list) ? list : []);
    } catch {
      setEvidence([]);
    }
  };

  const clearMessages = () => {
    setError(null);
    setSuccess(null);
  };

  const handleAddManualEvidence = async (e) => {
    e.preventDefault();
    if (!selectedControl || !manualType.trim()) return;
    setManualSubmitting(true);
    clearMessages();
    try {
      await compAiApi.postEvidence({
        control_id: selectedControl.id,
        type: manualType.trim(),
        source: manualSource.trim() || undefined,
        description: manualDescription.trim() || undefined,
        link_url: manualLinkUrl.trim() || undefined,
      });
      setSuccess('Evidence added.');
      setManualType('');
      setManualSource('');
      setManualDescription('');
      setManualLinkUrl('');
      loadEvidence(selectedControl.id);
    } catch (err) {
      setError(err.message || 'Failed to add evidence');
    } finally {
      setManualSubmitting(false);
    }
  };

  const handleCollectGitHubEvidence = async (e) => {
    e.preventDefault();
    if (!selectedControl || !ghOwner.trim() || !ghRepo.trim()) return;
    setGhSubmitting(true);
    clearMessages();
    try {
      await compAiApi.postGitHubEvidence({
        control_id: selectedControl.id,
        owner: ghOwner.trim(),
        repo: ghRepo.trim(),
        evidence_type: ghEvidenceType,
      });
      setSuccess('GitHub evidence collected and linked.');
      loadEvidence(selectedControl.id);
    } catch (err) {
      setError(err.message || 'Failed to collect GitHub evidence');
    } finally {
      setGhSubmitting(false);
    }
  };

  const formatDate = (dateString) => {
    if (!dateString) return 'N/A';
    try {
      return new Date(dateString).toLocaleString();
    } catch {
      return dateString;
    }
  };

  return (
    <PageTemplate
      title="Controls & Evidence"
      subtitle="Compliance controls and evidence (Phase 4)"
    >
      <div className="comp-ai-container comp-ai-controls-container">
        <div className="comp-ai-controls-main">
          {error && (
            <div className="comp-ai-error" role="alert">
              {error}
            </div>
          )}
          {success && (
            <div className="comp-ai-success" role="status">
              {success}
            </div>
          )}

          <div className="comp-ai-controls-section">
            <h2>Controls</h2>
            {loading && <div className="comp-ai-loading">Loading controls...</div>}
            {!loading && controls.length === 0 && (
              <p className="comp-ai-empty">No controls found. Run Phase 4 migrations (009, 010, 011) on the database.</p>
            )}
            {!loading && controls.length > 0 && (
              <ul className="comp-ai-controls-list">
                {controls.map((c) => (
                  <li key={c.id}>
                    <button
                      type="button"
                      className={`comp-ai-control-item ${selectedControl?.id === c.id ? 'selected' : ''}`}
                      onClick={() => setSelectedControl(c)}
                    >
                      <span className="comp-ai-control-id">{c.internal_id}</span>
                      <span className="comp-ai-control-name">{c.name}</span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>

          {selectedControl && (
            <>
              <div className="comp-ai-controls-section comp-ai-control-detail">
                <h2>Control details</h2>
                {detailLoading && <div className="comp-ai-loading">Loading...</div>}
                {!detailLoading && controlDetail && (
                  <>
                    <p><strong>{controlDetail.internal_id}</strong> — {controlDetail.name}</p>
                    {controlDetail.description && <p className="comp-ai-control-desc">{controlDetail.description}</p>}
                    {controlDetail.requirements && controlDetail.requirements.length > 0 && (
                      <div className="comp-ai-requirements">
                        <h3>Mapped requirements</h3>
                        <ul>
                          {controlDetail.requirements.map((r, i) => (
                            <li key={i}>
                              {r.framework_slug}: {r.code} — {r.title || '—'}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}
                  </>
                )}
              </div>

              <div className="comp-ai-controls-section">
                <h2>Evidence</h2>
                {evidence.length === 0 && <p className="comp-ai-empty">No evidence for this control yet.</p>}
                {evidence.length > 0 && (
                  <ul className="comp-ai-evidence-list">
                    {evidence.map((ev) => (
                      <li key={ev.id} className="comp-ai-evidence-item">
                        <span className="comp-ai-evidence-type">{ev.type}</span>
                        {ev.source && <span> — {ev.source}</span>}
                        {ev.description && <p className="comp-ai-evidence-desc">{ev.description}</p>}
                        {ev.link_url && (
                          <a href={ev.link_url} target="_blank" rel="noopener noreferrer" className="comp-ai-evidence-link">
                            Link
                          </a>
                        )}
                        <span className="comp-ai-evidence-date">{formatDate(ev.collected_at)}</span>
                      </li>
                    ))}
                  </ul>
                )}
              </div>

              <div className="comp-ai-controls-section comp-ai-forms">
                <h2>Add evidence</h2>

                <form onSubmit={handleAddManualEvidence} className="comp-ai-evidence-form">
                  <h3>Manual evidence</h3>
                  <div className="form-group">
                    <label htmlFor="manual-type">Type *</label>
                    <input
                      id="manual-type"
                      type="text"
                      value={manualType}
                      onChange={(e) => setManualType(e.target.value)}
                      placeholder="e.g. policy_doc, screenshot"
                      required
                    />
                  </div>
                  <div className="form-group">
                    <label htmlFor="manual-source">Source</label>
                    <input
                      id="manual-source"
                      type="text"
                      value={manualSource}
                      onChange={(e) => setManualSource(e.target.value)}
                      placeholder="e.g. Confluence, Google Drive"
                    />
                  </div>
                  <div className="form-group">
                    <label htmlFor="manual-description">Description</label>
                    <textarea
                      id="manual-description"
                      value={manualDescription}
                      onChange={(e) => setManualDescription(e.target.value)}
                      placeholder="Short description"
                      rows={2}
                    />
                  </div>
                  <div className="form-group">
                    <label htmlFor="manual-link">Link URL</label>
                    <input
                      id="manual-link"
                      type="url"
                      value={manualLinkUrl}
                      onChange={(e) => setManualLinkUrl(e.target.value)}
                      placeholder="https://..."
                    />
                  </div>
                  <button type="submit" className="comp-ai-submit-btn" disabled={manualSubmitting}>
                    {manualSubmitting ? 'Adding...' : 'Add evidence'}
                  </button>
                </form>

                <form onSubmit={handleCollectGitHubEvidence} className="comp-ai-evidence-form">
                  <h3>Collect from GitHub</h3>
                  <p className="comp-ai-form-hint">Requires GITHUB_TOKEN configured on the server.</p>
                  <div className="form-group">
                    <label htmlFor="gh-owner">Owner *</label>
                    <input
                      id="gh-owner"
                      type="text"
                      value={ghOwner}
                      onChange={(e) => setGhOwner(e.target.value)}
                      placeholder="org or username"
                      required
                    />
                  </div>
                  <div className="form-group">
                    <label htmlFor="gh-repo">Repo *</label>
                    <input
                      id="gh-repo"
                      type="text"
                      value={ghRepo}
                      onChange={(e) => setGhRepo(e.target.value)}
                      placeholder="repository name"
                      required
                    />
                  </div>
                  <div className="form-group">
                    <label htmlFor="gh-type">Evidence type</label>
                    <select
                      id="gh-type"
                      value={ghEvidenceType}
                      onChange={(e) => setGhEvidenceType(e.target.value)}
                    >
                      <option value="last_commit">Last commit (default branch)</option>
                      <option value="branch_protection">Branch protection (default branch)</option>
                    </select>
                  </div>
                  <button type="submit" className="comp-ai-submit-btn" disabled={ghSubmitting}>
                    {ghSubmitting ? 'Collecting...' : 'Collect evidence'}
                  </button>
                </form>
              </div>
            </>
          )}
        </div>

        <div className="comp-ai-sidebar">
          <div className="comp-ai-info-card">
            <h3>Controls & Evidence</h3>
            <p>
              Select a control to view requirements and evidence. Add evidence manually or collect from GitHub
              (last commit or branch protection).
            </p>
          </div>
          <div className="comp-ai-info-card">
            <h3>Quick links</h3>
            <ul>
              <li><Link to="/comp-ai">Comp AI chat</Link></li>
              <li><Link to="/comp-ai/history">Request history</Link></li>
              <li><Link to="/comp-ai/monitoring">Monitoring</Link></li>
            </ul>
          </div>
        </div>
      </div>
    </PageTemplate>
  );
}

export default CompAIControls;
