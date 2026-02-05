import React, { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { compAiApi } from './compAiApi';
import PageTemplate from '../PageTemplate';
import { formatDateEU, formatDateTimeEU, formatFilenameDateEU, euStringFromISOInput, toISODateFromInput, EU_DATE_INPUT_PLACEHOLDER, EU_DATE_INPUT_LABEL } from '../services/dateFormat';
import './CompAI.css';

function CompAIControls() {
  const [controls, setControls] = useState([]);
  const [gaps, setGaps] = useState([]);
  const [gapsLoading, setGapsLoading] = useState(false);
  const [selectedControl, setSelectedControl] = useState(null);
  const [controlDetail, setControlDetail] = useState(null);
  const [evidence, setEvidence] = useState([]);
  const [loading, setLoading] = useState(true);
  const [detailLoading, setDetailLoading] = useState(false);
  const [exportLoading, setExportLoading] = useState(false);
  const [error, setError] = useState(null);
  const [success, setSuccess] = useState(null);

  // Manual evidence form (Phase A: document/policy/spreadsheet/email types)
  const EVIDENCE_TYPE_PRESETS = ['document', 'policy', 'spreadsheet', 'email', 'manual', 'other'];
  const [manualType, setManualType] = useState('document');
  const [manualTypeOther, setManualTypeOther] = useState('');
  const [manualSource, setManualSource] = useState('');
  const [manualDescription, setManualDescription] = useState('');
  const [manualLinkUrl, setManualLinkUrl] = useState('');
  const [manualSubmitting, setManualSubmitting] = useState(false);

  // GitHub evidence form
  const [ghOwner, setGhOwner] = useState('');
  const [ghRepo, setGhRepo] = useState('');
  const [ghEvidenceType, setGhEvidenceType] = useState('last_commit');
  const [ghSubmitting, setGhSubmitting] = useState(false);
  const [oktaEvidenceType, setOktaEvidenceType] = useState('org_summary');
  const [oktaSubmitting, setOktaSubmitting] = useState(false);
  const [awsEvidenceType, setAwsEvidenceType] = useState('iam_summary');
  const [awsSubmitting, setAwsSubmitting] = useState(false);

  // Phase B: file upload
  const [uploadFile, setUploadFile] = useState(null);
  const [uploadType, setUploadType] = useState('document');
  const [uploadSubmitting, setUploadSubmitting] = useState(false);
  // Phase B: document analyse
  const [analyzeResult, setAnalyzeResult] = useState(null);
  const [analyzeLoadingId, setAnalyzeLoadingId] = useState(null);
  const [analyzeError, setAnalyzeError] = useState(null);

  // Remediation (Phase 5)
  const [remediationTask, setRemediationTask] = useState(null);
  const [remediationLoading, setRemediationLoading] = useState(false);
  const [remediationSubmitting, setRemediationSubmitting] = useState(false);
  const [remediationAssignedTo, setRemediationAssignedTo] = useState('');
  const [remediationDueDate, setRemediationDueDate] = useState('');
  const [remediationStatus, setRemediationStatus] = useState('open');
  const [remediationNotes, setRemediationNotes] = useState('');
  // Phase 7.1: AI remediation suggestion
  const [remediationSuggest, setRemediationSuggest] = useState(null);
  const [remediationSuggestLoading, setRemediationSuggestLoading] = useState(false);
  const [remediationSuggestError, setRemediationSuggestError] = useState(null);
  // Phase 7.2: AI gap/risk analysis
  const [gapAnalysis, setGapAnalysis] = useState(null);
  const [gapAnalysisLoading, setGapAnalysisLoading] = useState(false);
  const [gapAnalysisError, setGapAnalysisError] = useState(null);
  // G9: AI evidence review (all evidence for this control)
  const [evidenceReview, setEvidenceReview] = useState(null);
  const [evidenceReviewLoading, setEvidenceReviewLoading] = useState(false);
  const [evidenceReviewError, setEvidenceReviewError] = useState(null);
  const [externalIdEdit, setExternalIdEdit] = useState('');
  const [externalIdSubmitting, setExternalIdSubmitting] = useState(false);

  // Phase C: Scan documents (batch URLs + batch upload)
  const [scanUrls, setScanUrls] = useState('');
  const [scanType, setScanType] = useState('document');
  const [scanUrlsSubmitting, setScanUrlsSubmitting] = useState(false);
  const [scanBatchFiles, setScanBatchFiles] = useState([]);
  const [scanBatchType, setScanBatchType] = useState('document');
  const [scanBatchSubmitting, setScanBatchSubmitting] = useState(false);
  const [scanResult, setScanResult] = useState(null);
  const [scanError, setScanError] = useState(null);

  useEffect(() => {
    loadControls();
  }, []);

  useEffect(() => {
    if (selectedControl) {
      loadControlDetail(selectedControl.id);
      loadEvidence(selectedControl.id);
      loadRemediation(selectedControl.id);
      setEvidenceReview(null);
      setEvidenceReviewError(null);
    } else {
      setControlDetail(null);
      setEvidence([]);
      setRemediationTask(null);
      setRemediationSuggest(null);
      setRemediationSuggestError(null);
      setEvidenceReview(null);
      setEvidenceReviewError(null);
    }
  }, [selectedControl]);

  useEffect(() => {
    setExternalIdEdit(controlDetail?.external_id ?? '');
  }, [controlDetail]);

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

  const loadRemediation = async (controlId) => {
    setRemediationLoading(true);
    try {
      const task = await compAiApi.getControlRemediation(controlId);
      setRemediationTask(task);
      if (task) {
        setRemediationAssignedTo(task.assigned_to || '');
        setRemediationDueDate(task.due_date ? euStringFromISOInput(task.due_date.slice(0, 10)) : '');
        setRemediationStatus(task.status || 'open');
        setRemediationNotes(task.notes || '');
      } else {
        setRemediationAssignedTo('');
        setRemediationDueDate('');
        setRemediationStatus('open');
        setRemediationNotes('');
      }
    } catch {
      setRemediationTask(null);
    } finally {
      setRemediationLoading(false);
    }
  };

  const handleSaveRemediation = async (e) => {
    e.preventDefault();
    if (!selectedControl) return;
    setRemediationSubmitting(true);
    clearMessages();
    try {
      await compAiApi.putControlRemediation(selectedControl.id, {
        assigned_to: remediationAssignedTo.trim() || undefined,
        due_date: toISODateFromInput(remediationDueDate) || undefined,
        status: remediationStatus,
        notes: remediationNotes.trim() || undefined,
      });
      setSuccess('Remediation saved.');
      loadRemediation(selectedControl.id);
    } catch (err) {
      setError(err.message || 'Failed to save remediation');
    } finally {
      setRemediationSubmitting(false);
    }
  };

  const handleGetRemediationSuggest = async () => {
    if (!selectedControl) return;
    setRemediationSuggestLoading(true);
    setRemediationSuggestError(null);
    setRemediationSuggest(null);
    try {
      const data = await compAiApi.postRemediationSuggest(selectedControl.id, {});
      setRemediationSuggest(data);
    } catch (err) {
      setRemediationSuggestError(err.message || 'Failed to get AI suggestion');
    } finally {
      setRemediationSuggestLoading(false);
    }
  };

  const handleUseSuggestionInNotes = () => {
    if (remediationSuggest?.suggestion) {
      setRemediationNotes((prev) => (prev ? `${prev}\n\n${remediationSuggest.suggestion}` : remediationSuggest.suggestion));
    }
  };

  /** Phase 7.3: One-click apply – save suggestion into remediation (notes) and persist. */
  const handleApplySuggestionToRemediation = async () => {
    if (!selectedControl || !remediationSuggest?.suggestion) return;
    setRemediationSubmitting(true);
    clearMessages();
    const newNotes = (remediationNotes.trim() ? remediationNotes.trim() + '\n\n' : '') + remediationSuggest.suggestion;
    try {
      await compAiApi.putControlRemediation(selectedControl.id, {
        assigned_to: remediationAssignedTo.trim() || undefined,
        due_date: toISODateFromInput(remediationDueDate) || undefined,
        status: remediationStatus,
        notes: newNotes || undefined,
      });
      setRemediationNotes(newNotes);
      setRemediationSuggest(null);
      setSuccess('AI suggestion applied to remediation.');
      loadRemediation(selectedControl.id);
    } catch (err) {
      setError(err.message || 'Failed to apply suggestion');
    } finally {
      setRemediationSubmitting(false);
    }
  };

  const handleAnalyseGaps = async () => {
    setGapAnalysisLoading(true);
    setGapAnalysisError(null);
    setGapAnalysis(null);
    try {
      const data = await compAiApi.postAnalysisGaps({});
      setGapAnalysis(data);
    } catch (err) {
      setGapAnalysisError(err.message || 'Failed to analyse gaps');
    } finally {
      setGapAnalysisLoading(false);
    }
  };

  const loadGaps = async () => {
    setGapsLoading(true);
    setError(null);
    try {
      const list = await compAiApi.getControlsGaps();
      setGaps(Array.isArray(list) ? list : []);
    } catch (err) {
      setError(err.message || 'Failed to load gaps');
    } finally {
      setGapsLoading(false);
    }
  };

  const handleReviewEvidence = async () => {
    if (!selectedControl) return;
    setEvidenceReviewLoading(true);
    setEvidenceReviewError(null);
    setEvidenceReview(null);
    try {
      const data = await compAiApi.postControlEvidenceReview(selectedControl.id);
      setEvidenceReview(data);
    } catch (err) {
      setEvidenceReviewError(err.message || 'Failed to review evidence');
    } finally {
      setEvidenceReviewLoading(false);
    }
  };

  const handleDownloadAuditExport = async () => {
    setExportLoading(true);
    clearMessages();
    try {
      const data = await compAiApi.getControlsExport();
      const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `comp-ai-audit-export-${formatFilenameDateEU()}.json`;
      a.click();
      URL.revokeObjectURL(url);
      setSuccess('Audit export downloaded.');
    } catch (err) {
      setError(err.message || 'Failed to download audit export');
    } finally {
      setExportLoading(false);
    }
  };

  const handleDownloadCsvExport = async () => {
    setExportLoading(true);
    clearMessages();
    try {
      const blob = await compAiApi.getControlsExport('csv');
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `comp-ai-audit-export-${formatFilenameDateEU()}.csv`;
      a.click();
      URL.revokeObjectURL(url);
      setSuccess('CSV export downloaded.');
    } catch (err) {
      setError(err.message || 'Failed to download CSV export');
    } finally {
      setExportLoading(false);
    }
  };

  const handlePrintExport = async () => {
    setExportLoading(true);
    clearMessages();
    try {
      const data = await compAiApi.getControlsExport();
      const win = window.open('', '_blank');
      if (!win) {
        setError('Please allow pop-ups to print.');
        return;
      }
      const rows = (data.controls || []).map((e) => {
        const reqs = (e.control?.requirements || []).map((r) => `${r.framework_slug}:${r.code}`).join(', ');
        const evCount = (e.evidence || []).length;
        const evTypes = (e.evidence || []).map((ev) => ev.type).join(', ');
        return `<tr><td>${e.control?.internal_id ?? ''}</td><td>${e.control?.name ?? ''}</td><td>${reqs}</td><td>${evCount}</td><td>${evTypes}</td></tr>`;
      }).join('');
      win.document.write(`
        <!DOCTYPE html><html><head><title>Comp-AI Audit Export</title>
        <style>body{font-family:sans-serif;padding:20px;} table{border-collapse:collapse;width:100%;} th,td{border:1px solid #ccc;padding:8px;text-align:left;} th{background:#eee;}</style>
        </head><body>
        <h1>Comp-AI Audit Export</h1>
        <p>Exported: ${data.exported_at ? formatDateTimeEU(data.exported_at) : formatDateTimeEU(new Date())}</p>
        <table>
        <thead><tr><th>Control ID</th><th>Name</th><th>Requirements</th><th>Evidence count</th><th>Evidence types</th></tr></thead>
        <tbody>${rows}</tbody>
        </table>
        </body></html>
      `);
      win.document.close();
      win.focus();
      setTimeout(() => { win.print(); win.close(); }, 250);
      setSuccess('Print dialog opened. Use "Save as PDF" to save as PDF.');
    } catch (err) {
      setError(err.message || 'Failed to open print view');
    } finally {
      setExportLoading(false);
    }
  };

  const clearMessages = () => {
    setError(null);
    setSuccess(null);
  };

  const handleAddManualEvidence = async (e) => {
    e.preventDefault();
    const typeValue = manualType === 'other' ? manualTypeOther.trim() : manualType;
    if (!selectedControl || !typeValue) return;
    setManualSubmitting(true);
    clearMessages();
    try {
      await compAiApi.postEvidence({
        control_id: selectedControl.id,
        type: typeValue,
        source: manualSource.trim() || undefined,
        description: manualDescription.trim() || undefined,
        link_url: manualLinkUrl.trim() || undefined,
      });
      setSuccess('Evidence added.');
      setManualType('document');
      setManualTypeOther('');
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

  const handleCollectOktaEvidence = async (e) => {
    e.preventDefault();
    if (!selectedControl) return;
    setOktaSubmitting(true);
    clearMessages();
    try {
      await compAiApi.postOktaEvidence({
        control_id: selectedControl.id,
        evidence_type: oktaEvidenceType,
      });
      setSuccess('Okta evidence collected and linked.');
      loadEvidence(selectedControl.id);
    } catch (err) {
      setError(err.message || 'Failed to collect Okta evidence');
    } finally {
      setOktaSubmitting(false);
    }
  };

  const handleCollectAwsEvidence = async (e) => {
    e.preventDefault();
    if (!selectedControl) return;
    setAwsSubmitting(true);
    clearMessages();
    try {
      await compAiApi.postAwsEvidence({
        control_id: selectedControl.id,
        evidence_type: awsEvidenceType,
      });
      setSuccess('AWS evidence collected and linked.');
      loadEvidence(selectedControl.id);
    } catch (err) {
      setError(err.message || 'Failed to collect AWS evidence');
    } finally {
      setAwsSubmitting(false);
    }
  };

  const handleScanUrls = async (e) => {
    e.preventDefault();
    if (!selectedControl || !scanUrls.trim()) return;
    const urls = scanUrls.trim().split(/\n/).map((s) => s.trim()).filter(Boolean);
    if (urls.length === 0) return;
    setScanUrlsSubmitting(true);
    setScanError(null);
    setScanResult(null);
    clearMessages();
    try {
      const documents = urls.map((url) => ({ url, type: scanType }));
      const data = await compAiApi.postScanDocuments(selectedControl.id, documents);
      setScanResult(data);
      setSuccess(`Scan: created ${data.created} evidence item(s).`);
      setScanUrls('');
      loadEvidence(selectedControl.id);
    } catch (err) {
      setScanError(err.message || 'Scan failed');
    } finally {
      setScanUrlsSubmitting(false);
    }
  };

  const handleScanBatch = async (e) => {
    e.preventDefault();
    if (!selectedControl || !scanBatchFiles.length) return;
    setScanBatchSubmitting(true);
    setScanError(null);
    setScanResult(null);
    clearMessages();
    try {
      const files = Array.from(scanBatchFiles);
      const data = await compAiApi.postScanUploadBatch(selectedControl.id, scanBatchType, files);
      setScanResult(data);
      setSuccess(`Batch upload: created ${data.created} evidence item(s).`);
      setScanBatchFiles([]);
      const input = document.getElementById('scan-batch-files');
      if (input) input.value = '';
      loadEvidence(selectedControl.id);
    } catch (err) {
      setScanError(err.message || 'Batch upload failed');
    } finally {
      setScanBatchSubmitting(false);
    }
  };

  const handleUploadEvidence = async (e) => {
    e.preventDefault();
    if (!selectedControl || !uploadFile) return;
    setUploadSubmitting(true);
    setAnalyzeError(null);
    setAnalyzeResult(null);
    clearMessages();
    try {
      await compAiApi.postEvidenceUpload(selectedControl.id, uploadType, uploadFile);
      setSuccess('File uploaded and linked as evidence.');
      setUploadFile(null);
      loadEvidence(selectedControl.id);
    } catch (err) {
      setError(err.message || 'Upload failed');
    } finally {
      setUploadSubmitting(false);
    }
  };

  const handleAnalyzeEvidence = async (ev) => {
    setAnalyzeLoadingId(ev.id);
    setAnalyzeError(null);
    setAnalyzeResult(null);
    try {
      const data = await compAiApi.postEvidenceAnalyze(ev.id);
      setAnalyzeResult({ evidenceId: ev.id, ...data });
    } catch (err) {
      setAnalyzeError(err.message || 'Analysis failed');
    } finally {
      setAnalyzeLoadingId(null);
    }
  };

  const formatDate = (dateString) => {
    if (!dateString) return 'N/A';
    return formatDateTimeEU(dateString);
  };

  return (
    <PageTemplate
      title="Controls & Evidence"
      subtitle="Compliance controls, evidence, and audit export (Phase 4 & 5)"
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

          <div className="comp-ai-controls-section comp-ai-audit-actions">
            <h2>Audit (Phase 5)</h2>
            <p className="comp-ai-form-hint">Download a full export for auditors. View controls that have no evidence (gaps).</p>
            <div className="comp-ai-button-row">
              <button
                type="button"
                className="comp-ai-submit-btn"
                onClick={handleDownloadAuditExport}
                disabled={exportLoading}
              >
                {exportLoading ? 'Preparing...' : 'Download JSON'}
              </button>
              <button
                type="button"
                className="comp-ai-secondary-btn"
                onClick={handleDownloadCsvExport}
                disabled={exportLoading}
              >
                {exportLoading ? 'Preparing...' : 'Download CSV'}
              </button>
              <button
                type="button"
                className="comp-ai-secondary-btn"
                onClick={handlePrintExport}
                disabled={exportLoading}
              >
                {exportLoading ? 'Preparing...' : 'Print / Save as PDF'}
              </button>
              <button
                type="button"
                className="comp-ai-secondary-btn"
                onClick={loadGaps}
                disabled={gapsLoading}
              >
                {gapsLoading ? 'Loading...' : 'Show gaps'}
              </button>
            </div>
            {gaps.length > 0 && (
              <div className="comp-ai-gaps-section">
                <h3>Controls with no evidence ({gaps.length})</h3>
                <button
                  type="button"
                  className="comp-ai-secondary-btn comp-ai-analyse-gaps-btn"
                  onClick={handleAnalyseGaps}
                  disabled={gapAnalysisLoading}
                >
                  {gapAnalysisLoading ? 'Analysing...' : 'Analyse my gaps'}
                </button>
                {gapAnalysisError && (
                  <p className="comp-ai-error" role="alert">{gapAnalysisError}</p>
                )}
                {gapAnalysis && (
                  <div className="comp-ai-suggestion-card comp-ai-gap-analysis-card">
                    <p className="comp-ai-suggestion-meta">Model: {gapAnalysis.model_used}</p>
                    <div className="comp-ai-suggestion-text">{gapAnalysis.summary}</div>
                  </div>
                )}
                <ul className="comp-ai-controls-list comp-ai-gaps-list">
                  {gaps.map((c) => (
                    <li key={c.id}>
                      <button
                        type="button"
                        className="comp-ai-control-item"
                        onClick={() => setSelectedControl(c)}
                      >
                        <span className="comp-ai-control-id">{c.internal_id}</span>
                        <span className="comp-ai-control-name">{c.name}</span>
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>

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
                    <div className="comp-ai-external-id">
                      <label htmlFor="external-id">External ID</label>
                      <input
                        id="external-id"
                        type="text"
                        value={externalIdEdit}
                        onChange={(e) => setExternalIdEdit(e.target.value)}
                        placeholder={controlDetail.external_id || 'e.g. SOC2-CC6.1 or framework control ID'}
                      />
                      <button
                        type="button"
                        className="comp-ai-secondary-btn"
                        disabled={externalIdSubmitting}
                        onClick={async () => {
                          if (!selectedControl) return;
                          setExternalIdSubmitting(true);
                          try {
                            await compAiApi.patchControl(selectedControl.id, {
                              external_id: externalIdEdit.trim() || null,
                            });
                            await loadControlDetail(selectedControl.id);
                          } catch (err) {
                            setError(err.message || 'Failed to update external ID');
                          } finally {
                            setExternalIdSubmitting(false);
                          }
                        }}
                      >
                        {externalIdSubmitting ? 'Saving...' : 'Save'}
                      </button>
                      {controlDetail.external_id && (
                        <span className="comp-ai-meta">Current: {controlDetail.external_id}</span>
                      )}
                    </div>
                    {controlDetail.requirements && controlDetail.requirements.length > 0 && (
                      <div className="comp-ai-requirements">
                        <h3>Mapped requirements</h3>
                        <ul>
                          {controlDetail.requirements.map((r, i) => (
                            <li key={i}>
                              {r.framework_slug}: {r.code}
                              {r.external_id && <span className="comp-ai-meta"> ({r.external_id})</span>}
                              {' — '}{r.title || '—'}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}
                  </>
                )}
              </div>

              <div className="comp-ai-controls-section comp-ai-remediation-section">
                <h2>Remediation (Phase 5)</h2>
                <p className="comp-ai-form-hint">Track gaps: assign owner, due date, and status (open / in progress / done).</p>
                {remediationLoading && <div className="comp-ai-loading">Loading...</div>}
                {!remediationLoading && (
                  <>
                    {/* Phase 7.1: AI remediation suggestion */}
                    <div className="comp-ai-remediation-suggest">
                      <button
                        type="button"
                        className="comp-ai-secondary-btn"
                        onClick={handleGetRemediationSuggest}
                        disabled={remediationSuggestLoading}
                      >
                        {remediationSuggestLoading ? 'Getting suggestion...' : 'Get AI suggestion'}
                      </button>
                      {remediationSuggestError && (
                        <p className="comp-ai-error" role="alert">{remediationSuggestError}</p>
                      )}
                      {remediationSuggest && (
                        <div className="comp-ai-suggestion-card">
                          <p className="comp-ai-suggestion-meta">Model: {remediationSuggest.model_used}</p>
                          <div className="comp-ai-suggestion-text">{remediationSuggest.suggestion}</div>
                          <div className="comp-ai-suggestion-actions">
                            <button
                              type="button"
                              className="comp-ai-secondary-btn comp-ai-use-suggestion-btn"
                              onClick={handleUseSuggestionInNotes}
                            >
                              Use in notes
                            </button>
                            <button
                              type="button"
                              className="comp-ai-submit-btn"
                              onClick={handleApplySuggestionToRemediation}
                              disabled={remediationSubmitting}
                            >
                              {remediationSubmitting ? 'Applying...' : 'Apply to remediation'}
                            </button>
                          </div>
                        </div>
                      )}
                    </div>
                    {remediationTask && (
                      <div className="comp-ai-remediation-current">
                        <p><strong>Current:</strong> {remediationTask.assigned_to || '—'} · Due {remediationTask.due_date ? formatDateEU(remediationTask.due_date) : '—'} · {remediationTask.status}</p>
                        {remediationTask.notes && <p className="comp-ai-remediation-notes">{remediationTask.notes}</p>}
                      </div>
                    )}
                    <form onSubmit={handleSaveRemediation} className="comp-ai-evidence-form">
                      <div className="form-group">
                        <label htmlFor="remediation-assigned">Assigned to</label>
                        <input
                          id="remediation-assigned"
                          type="text"
                          value={remediationAssignedTo}
                          onChange={(e) => setRemediationAssignedTo(e.target.value)}
                          placeholder="Owner or team"
                        />
                      </div>
                      <div className="form-group eu-date-input-wrapper">
                        <label htmlFor="remediation-due">Due date ({EU_DATE_INPUT_LABEL})</label>
                        <input
                          id="remediation-due"
                          type="text"
                          inputMode="text"
                          autoComplete="off"
                          data-date-format="eu"
                          value={remediationDueDate}
                          onChange={(e) => setRemediationDueDate(e.target.value)}
                          placeholder={EU_DATE_INPUT_PLACEHOLDER}
                          title={`Enter date as ${EU_DATE_INPUT_LABEL}`}
                          aria-label={`Due date in ${EU_DATE_INPUT_LABEL} format`}
                        />
                        <span className="comp-ai-form-hint comp-ai-date-hint">Format: {EU_DATE_INPUT_LABEL}</span>
                      </div>
                      <div className="form-group">
                        <label htmlFor="remediation-status">Status</label>
                        <select
                          id="remediation-status"
                          value={remediationStatus}
                          onChange={(e) => setRemediationStatus(e.target.value)}
                        >
                          <option value="open">Open</option>
                          <option value="in_progress">In progress</option>
                          <option value="done">Done</option>
                        </select>
                      </div>
                      <div className="form-group">
                        <label htmlFor="remediation-notes">Notes</label>
                        <textarea
                          id="remediation-notes"
                          value={remediationNotes}
                          onChange={(e) => setRemediationNotes(e.target.value)}
                          placeholder="Remediation notes"
                          rows={2}
                        />
                      </div>
                      <button type="submit" className="comp-ai-submit-btn" disabled={remediationSubmitting}>
                        {remediationSubmitting ? 'Saving...' : remediationTask ? 'Update remediation' : 'Start remediation'}
                      </button>
                    </form>
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
                        {ev.file_name && <span> ({ev.file_name})</span>}
                        {ev.description && <p className="comp-ai-evidence-desc">{ev.description}</p>}
                        {ev.link_url && (
                          <a href={ev.link_url} target="_blank" rel="noopener noreferrer" className="comp-ai-evidence-link">
                            Link
                          </a>
                        )}
                        <span className="comp-ai-evidence-date">{formatDate(ev.collected_at)}</span>
                        {(ev.extracted_text || ev.file_name) && (
                          <button
                            type="button"
                            className="comp-ai-secondary-btn comp-ai-analyze-btn"
                            onClick={() => handleAnalyzeEvidence(ev)}
                            disabled={analyzeLoadingId === ev.id}
                          >
                            {analyzeLoadingId === ev.id ? 'Analysing...' : 'Analyse'}
                          </button>
                        )}
                      </li>
                    ))}
                  </ul>
                )}
                {analyzeError && (
                  <div className="comp-ai-error" role="alert">{analyzeError}</div>
                )}
                {analyzeResult && (
                  <div className="comp-ai-analyze-result comp-ai-info-card">
                    <h4>Document analysis</h4>
                    <p className="comp-ai-analyze-summary">{analyzeResult.summary}</p>
                    {analyzeResult.model_used && (
                      <p className="comp-ai-analyze-model">Model: {analyzeResult.model_used}</p>
                    )}
                  </div>
                )}
                {evidence.length > 0 && (
                  <>
                    <button
                      type="button"
                      className="comp-ai-secondary-btn"
                      onClick={handleReviewEvidence}
                      disabled={evidenceReviewLoading}
                    >
                      {evidenceReviewLoading ? 'Reviewing...' : 'Review all evidence (AI)'}
                    </button>
                    {evidenceReviewError && (
                      <div className="comp-ai-error" role="alert">{evidenceReviewError}</div>
                    )}
                    {evidenceReview && (
                      <div className="comp-ai-analyze-result comp-ai-info-card">
                        <h4>Evidence review</h4>
                        <p className="comp-ai-analyze-summary">{evidenceReview.summary}</p>
                        {evidenceReview.gaps && evidenceReview.gaps.length > 0 && (
                          <div className="comp-ai-review-gaps">
                            <strong>Gaps:</strong>
                            <ul>
                              {evidenceReview.gaps.map((g, i) => (
                                <li key={i}>{g}</li>
                              ))}
                            </ul>
                          </div>
                        )}
                        {evidenceReview.suggested_fixes && evidenceReview.suggested_fixes.length > 0 && (
                          <div className="comp-ai-review-fixes">
                            <strong>Suggested fixes:</strong>
                            <ul>
                              {evidenceReview.suggested_fixes.map((f, i) => (
                                <li key={i}>{f}</li>
                              ))}
                            </ul>
                          </div>
                        )}
                        {evidenceReview.model_used && (
                          <p className="comp-ai-analyze-model">Model: {evidenceReview.model_used}</p>
                        )}
                      </div>
                    )}
                  </>
                )}
              </div>

              <div className="comp-ai-controls-section comp-ai-forms">
                <h2>Add evidence</h2>

                <form onSubmit={handleAddManualEvidence} className="comp-ai-evidence-form">
                  <h3>Manual evidence</h3>
                  <p className="comp-ai-form-hint">For operational/admin: use Document, Policy, Spreadsheet, or Email and paste a link (SharePoint, Drive, etc.).</p>
                  <div className="form-group">
                    <label htmlFor="manual-type">Type *</label>
                    <select
                      id="manual-type"
                      value={EVIDENCE_TYPE_PRESETS.includes(manualType) ? manualType : 'other'}
                      onChange={(e) => setManualType(e.target.value)}
                      required
                    >
                      <option value="document">Document</option>
                      <option value="policy">Policy</option>
                      <option value="spreadsheet">Spreadsheet</option>
                      <option value="email">Email</option>
                      <option value="manual">Manual / other technical</option>
                      <option value="other">Other (custom)</option>
                    </select>
                  </div>
                  {manualType === 'other' && (
                    <div className="form-group">
                      <label htmlFor="manual-type-other">Custom type *</label>
                      <input
                        id="manual-type-other"
                        type="text"
                        value={manualTypeOther}
                        onChange={(e) => setManualTypeOther(e.target.value)}
                        placeholder="e.g. screenshot, policy_doc"
                        required={manualType === 'other'}
                      />
                    </div>
                  )}
                  <div className="form-group">
                    <label htmlFor="manual-source">Source</label>
                    <input
                      id="manual-source"
                      type="text"
                      value={manualSource}
                      onChange={(e) => setManualSource(e.target.value)}
                      placeholder="e.g. Confluence, Google Drive, SharePoint"
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
                      placeholder="https://... (e.g. SharePoint, Drive)"
                    />
                  </div>
                  <button type="submit" className="comp-ai-submit-btn" disabled={manualSubmitting}>
                    {manualSubmitting ? 'Adding...' : 'Add evidence'}
                  </button>
                </form>

                <form onSubmit={handleUploadEvidence} className="comp-ai-evidence-form">
                  <h3>Upload file (Phase B)</h3>
                  <p className="comp-ai-form-hint">PDF or text file. Text is extracted for AI analysis. Requires COMP_AI_EVIDENCE_STORAGE_PATH on server.</p>
                  <div className="form-group">
                    <label htmlFor="upload-type">Type</label>
                    <select
                      id="upload-type"
                      value={uploadType}
                      onChange={(e) => setUploadType(e.target.value)}
                    >
                      <option value="document">Document</option>
                      <option value="policy">Policy</option>
                      <option value="spreadsheet">Spreadsheet</option>
                    </select>
                  </div>
                  <div className="form-group">
                    <label htmlFor="upload-file">File *</label>
                    <input
                      id="upload-file"
                      type="file"
                      accept=".pdf,.txt,.csv"
                      onChange={(e) => setUploadFile(e.target.files?.[0] || null)}
                      required
                    />
                  </div>
                  <button type="submit" className="comp-ai-submit-btn" disabled={uploadSubmitting || !uploadFile}>
                    {uploadSubmitting ? 'Uploading...' : 'Upload file'}
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

                <form onSubmit={handleCollectOktaEvidence} className="comp-ai-evidence-form">
                  <h3>Collect from Okta (IdP) — G3</h3>
                  <p className="comp-ai-form-hint">Requires OKTA_DOMAIN and OKTA_API_TOKEN configured on the server.</p>
                  <div className="form-group">
                    <label htmlFor="okta-type">Evidence type</label>
                    <select
                      id="okta-type"
                      value={oktaEvidenceType}
                      onChange={(e) => setOktaEvidenceType(e.target.value)}
                    >
                      <option value="org_summary">Org summary</option>
                      <option value="users_snapshot">Users snapshot</option>
                      <option value="groups_snapshot">Groups snapshot</option>
                    </select>
                  </div>
                  <button type="submit" className="comp-ai-submit-btn" disabled={oktaSubmitting}>
                    {oktaSubmitting ? 'Collecting...' : 'Collect Okta evidence'}
                  </button>
                </form>

                <form onSubmit={handleCollectAwsEvidence} className="comp-ai-evidence-form">
                  <h3>Collect from AWS (IAM) — G4</h3>
                  <p className="comp-ai-form-hint">Requires AWS credentials (env or AWS_REGION). Uses default credential chain.</p>
                  <div className="form-group">
                    <label htmlFor="aws-type">Evidence type</label>
                    <select
                      id="aws-type"
                      value={awsEvidenceType}
                      onChange={(e) => setAwsEvidenceType(e.target.value)}
                    >
                      <option value="iam_summary">IAM account summary</option>
                    </select>
                  </div>
                  <button type="submit" className="comp-ai-submit-btn" disabled={awsSubmitting}>
                    {awsSubmitting ? 'Collecting...' : 'Collect AWS evidence'}
                  </button>
                </form>

                <div className="comp-ai-evidence-form">
                  <h3>Scan documents (Phase C)</h3>
                  <p className="comp-ai-form-hint">Batch add evidence: paste URLs (one per line) or upload multiple files. Select a control above first.</p>
                  {scanError && <p className="comp-ai-error">{scanError}</p>}
                  {scanResult && (
                    <p className="comp-ai-success">
                      Created {scanResult.created} evidence item(s). IDs: {scanResult.evidence_ids?.join(', ') || '—'}
                    </p>
                  )}
                  <form onSubmit={handleScanUrls}>
                    <h4>From URLs</h4>
                    <div className="form-group">
                      <label htmlFor="scan-urls">URLs (one per line) *</label>
                      <textarea
                        id="scan-urls"
                        value={scanUrls}
                        onChange={(e) => setScanUrls(e.target.value)}
                        placeholder={'https://example.com/policy.pdf\nhttps://example.com/sop.docx'}
                        rows={4}
                      />
                    </div>
                    <div className="form-group">
                      <label htmlFor="scan-type">Type</label>
                      <select id="scan-type" value={scanType} onChange={(e) => setScanType(e.target.value)}>
                        <option value="document">Document</option>
                        <option value="policy">Policy</option>
                        <option value="spreadsheet">Spreadsheet</option>
                      </select>
                    </div>
                    <button type="submit" className="comp-ai-submit-btn" disabled={scanUrlsSubmitting || !scanUrls.trim()}>
                      {scanUrlsSubmitting ? 'Scanning...' : 'Scan URLs'}
                    </button>
                  </form>
                  <form onSubmit={handleScanBatch} style={{ marginTop: '1rem' }}>
                    <h4>Upload multiple files</h4>
                    <div className="form-group">
                      <label htmlFor="scan-batch-type">Type</label>
                      <select id="scan-batch-type" value={scanBatchType} onChange={(e) => setScanBatchType(e.target.value)}>
                        <option value="document">Document</option>
                        <option value="policy">Policy</option>
                        <option value="spreadsheet">Spreadsheet</option>
                      </select>
                    </div>
                    <div className="form-group">
                      <label htmlFor="scan-batch-files">Files * (max 20)</label>
                      <input
                        id="scan-batch-files"
                        type="file"
                        multiple
                        accept=".pdf,.txt,.csv"
                        onChange={(e) => setScanBatchFiles(e.target.files ? Array.from(e.target.files) : [])}
                      />
                    </div>
                    <button type="submit" className="comp-ai-submit-btn" disabled={scanBatchSubmitting || scanBatchFiles.length === 0}>
                      {scanBatchSubmitting ? 'Uploading...' : `Upload ${scanBatchFiles.length || 0} file(s)`}
                    </button>
                  </form>
                </div>
              </div>
            </>
          )}
        </div>

        <div className="comp-ai-sidebar">
          <div className="comp-ai-info-card">
            <h3>Controls & Evidence</h3>
            <p>
              Select a control to view requirements and evidence. Add evidence manually or collect from GitHub,
              Okta (IdP), or AWS (IAM). Use <strong>Download for audit</strong> to export a full
              JSON for auditors; use <strong>Show gaps</strong> to list controls with no evidence.
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
