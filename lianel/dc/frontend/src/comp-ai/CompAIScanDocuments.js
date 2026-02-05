import React, { useState, useEffect, useCallback } from 'react';
import { Link } from 'react-router-dom';
import { compAiApi } from './compAiApi';
import PageTemplate from '../PageTemplate';
import './CompAI.css';

function CompAIScanDocuments() {
  const [controls, setControls] = useState([]);
  const [selectedControlId, setSelectedControlId] = useState('');
  const [loadingControls, setLoadingControls] = useState(true);
  const [controlsError, setControlsError] = useState(null);

  const [scanUrls, setScanUrls] = useState('');
  const [scanType, setScanType] = useState('document');
  const [scanUrlsSubmitting, setScanUrlsSubmitting] = useState(false);
  const [scanBatchType, setScanBatchType] = useState('document');
  const [scanBatchFiles, setScanBatchFiles] = useState([]);
  const [scanBatchSubmitting, setScanBatchSubmitting] = useState(false);

  const [scanError, setScanError] = useState(null);
  const [scanResult, setScanResult] = useState(null);

  const loadControls = useCallback(async () => {
    setLoadingControls(true);
    setControlsError(null);
    try {
      const list = await compAiApi.getControls();
      setControls(list);
      if (list.length > 0 && !selectedControlId) {
        setSelectedControlId(String(list[0].id));
      }
    } catch (e) {
      setControlsError(e.message || 'Failed to load controls');
    } finally {
      setLoadingControls(false);
    }
  }, [selectedControlId]);

  useEffect(() => {
    loadControls();
  }, [loadControls]);

  const handleScanUrls = async (e) => {
    e.preventDefault();
    if (!selectedControlId) {
      setScanError('Select a control first');
      return;
    }
    const urls = scanUrls
      .split(/\n/)
      .map((s) => s.trim())
      .filter(Boolean);
    if (urls.length === 0) {
      setScanError('Enter at least one URL');
      return;
    }
    setScanError(null);
    setScanResult(null);
    setScanUrlsSubmitting(true);
    try {
      const documents = urls.map((url) => ({ url, type: scanType }));
      const data = await compAiApi.postScanDocuments(Number(selectedControlId), documents);
      setScanResult(data);
    } catch (err) {
      setScanError(err.message || 'Scan failed');
    } finally {
      setScanUrlsSubmitting(false);
    }
  };

  const handleScanBatch = async (e) => {
    e.preventDefault();
    if (!selectedControlId) {
      setScanError('Select a control first');
      return;
    }
    if (scanBatchFiles.length === 0) {
      setScanError('Select at least one file');
      return;
    }
    setScanError(null);
    setScanResult(null);
    setScanBatchSubmitting(true);
    try {
      const data = await compAiApi.postScanUploadBatch(
        Number(selectedControlId),
        scanBatchType,
        scanBatchFiles
      );
      setScanResult(data);
      setScanBatchFiles([]);
    } catch (err) {
      setScanError(err.message || 'Batch upload failed');
    } finally {
      setScanBatchSubmitting(false);
    }
  };

  const selectedControl = controls.find((c) => String(c.id) === selectedControlId);

  return (
    <PageTemplate
      title="Scan documents"
      subtitle="Batch add evidence from URLs or upload multiple files"
    >
      <div className="comp-ai-container comp-ai-scan-container">
        <div className="comp-ai-scan-main">
          {loadingControls && <div className="comp-ai-loading">Loading controls...</div>}
          {controlsError && <p className="comp-ai-error" role="alert">{controlsError}</p>}
          {!loadingControls && !controlsError && controls.length === 0 && (
            <p className="comp-ai-empty">No controls found. Add controls first in Controls &amp; Evidence.</p>
          )}

          {!loadingControls && !controlsError && controls.length > 0 && (
            <>
              <section className="comp-ai-controls-section comp-ai-scan-section">
                <h2>Select control</h2>
                <p className="comp-ai-form-hint">
                  All evidence from the scan will be linked to the selected control.
                </p>
                <label>
                  <span>Control</span>
                  <select
                    value={selectedControlId}
                    onChange={(e) => setSelectedControlId(e.target.value)}
                    className="comp-ai-select comp-ai-scan-control-select"
                  >
                    {controls.map((c) => (
                      <option key={c.id} value={String(c.id)}>
                        {c.internal_id} — {c.name}
                      </option>
                    ))}
                  </select>
                </label>
              </section>

              <section className="comp-ai-controls-section comp-ai-scan-section">
                <h2>From URLs</h2>
                <p className="comp-ai-form-hint">Paste one URL per line. Max 50 per run.</p>
                {scanError && <p className="comp-ai-error" role="alert">{scanError}</p>}
                {scanResult && (
                  <div className="comp-ai-scan-result comp-ai-success" role="status">
                    <p><strong>Created {scanResult.created} evidence item(s).</strong></p>
                    {scanResult.evidence_ids && scanResult.evidence_ids.length > 0 && (
                      <p className="comp-ai-meta">Evidence IDs: {scanResult.evidence_ids.join(', ')}</p>
                    )}
                    {selectedControl && (
                      <p>
                        <Link to="/comp-ai/controls">View control &quot;{selectedControl.internal_id}&quot;</Link>
                      </p>
                    )}
                  </div>
                )}
                <form onSubmit={handleScanUrls} className="comp-ai-evidence-form">
                  <label htmlFor="scan-urls">URLs (one per line) *</label>
                  <textarea
                    id="scan-urls"
                    value={scanUrls}
                    onChange={(e) => setScanUrls(e.target.value)}
                    placeholder={'https://example.com/policy.pdf\nhttps://example.com/sop.docx'}
                    rows={5}
                    className="comp-ai-textarea"
                  />
                  <label>
                    <span>Type</span>
                    <select value={scanType} onChange={(e) => setScanType(e.target.value)} className="comp-ai-select">
                      <option value="document">Document</option>
                      <option value="policy">Policy</option>
                      <option value="spreadsheet">Spreadsheet</option>
                    </select>
                  </label>
                  <button
                    type="submit"
                    className="comp-ai-submit-btn"
                    disabled={scanUrlsSubmitting || !scanUrls.trim()}
                  >
                    {scanUrlsSubmitting ? 'Scanning...' : 'Scan URLs'}
                  </button>
                </form>
              </section>

              <section className="comp-ai-controls-section comp-ai-scan-section">
                <h2>Upload multiple files</h2>
                <p className="comp-ai-form-hint">PDF, text, or CSV. Max 20 files per run. Text is extracted for AI analysis.</p>
                <form onSubmit={handleScanBatch} className="comp-ai-evidence-form">
                  <label>
                    <span>Type</span>
                    <select
                      value={scanBatchType}
                      onChange={(e) => setScanBatchType(e.target.value)}
                      className="comp-ai-select"
                    >
                      <option value="document">Document</option>
                      <option value="policy">Policy</option>
                      <option value="spreadsheet">Spreadsheet</option>
                    </select>
                  </label>
                  <label>
                    <span>Files * (max 20)</span>
                    <input
                      type="file"
                      multiple
                      accept=".pdf,.txt,.csv"
                      onChange={(e) => setScanBatchFiles(e.target.files ? Array.from(e.target.files) : [])}
                      className="comp-ai-file-input"
                    />
                    {scanBatchFiles.length > 0 && (
                      <span className="comp-ai-meta">{scanBatchFiles.length} file(s) selected</span>
                    )}
                  </label>
                  <button
                    type="submit"
                    className="comp-ai-submit-btn"
                    disabled={scanBatchSubmitting || scanBatchFiles.length === 0}
                  >
                    {scanBatchSubmitting ? 'Uploading...' : `Upload ${scanBatchFiles.length || 0} file(s)`}
                  </button>
                </form>
              </section>
            </>
          )}
        </div>
        <div className="comp-ai-sidebar">
          <div className="comp-ai-info-card">
            <h3>Scan documents</h3>
            <p>
              Add evidence in bulk: paste document URLs or upload multiple files. All items are linked to the control you select.
              For automated weekly scans, set the Airflow Variable <code>COMP_AI_SCAN_DOCUMENTS_CONFIG</code>.
            </p>
          </div>
          <div className="comp-ai-info-card">
            <h3>Quick links</h3>
            <ul>
              <li><Link to="/comp-ai">Comp AI chat</Link></li>
              <li><Link to="/comp-ai/controls">Controls &amp; Evidence</Link></li>
              <li><Link to="/comp-ai/audit-docs">Audit docs</Link></li>
              <li><Link to="/comp-ai/history">Request history</Link></li>
              <li><Link to="/comp-ai/monitoring">Monitoring</Link></li>
            </ul>
          </div>
        </div>
      </div>
    </PageTemplate>
  );
}

export default CompAIScanDocuments;
