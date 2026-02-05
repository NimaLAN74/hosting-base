import React, { useState, useEffect, useCallback } from 'react';
import { Link } from 'react-router-dom';
import { compAiApi } from './compAiApi';
import PageTemplate from '../PageTemplate';
import './CompAI.css';

const PLACEHOLDERS = {
  '{{organisation_name}}': 'organisation_name',
  '{{system_name}}': 'system_name',
  '{{as_of_date}}': 'as_of_date',
};

function fillTemplate(template, values) {
  let out = template;
  for (const [placeholder, key] of Object.entries(PLACEHOLDERS)) {
    out = out.split(placeholder).join(values[key] ?? placeholder);
  }
  return out;
}

function CompAIAuditDocs() {
  const [systemDescription, setSystemDescription] = useState({ template: '', placeholders: [] });
  const [policyMapping, setPolicyMapping] = useState({ mapping: [] });
  const [loadingDesc, setLoadingDesc] = useState(true);
  const [loadingMapping, setLoadingMapping] = useState(true);
  const [errorDesc, setErrorDesc] = useState(null);
  const [errorMapping, setErrorMapping] = useState(null);
  const [fillValues, setFillValues] = useState({
    organisation_name: '',
    system_name: '',
    as_of_date: new Date().toISOString().slice(0, 10),
  });
  const [filledText, setFilledText] = useState('');
  const [copySuccess, setCopySuccess] = useState(false);

  const loadSystemDescription = useCallback(async () => {
    setLoadingDesc(true);
    setErrorDesc(null);
    try {
      const data = await compAiApi.getSystemDescription();
      setSystemDescription(data);
      setFilledText('');
    } catch (e) {
      setErrorDesc(e.message || 'Failed to load system description');
    } finally {
      setLoadingDesc(false);
    }
  }, []);

  const loadPolicyMapping = useCallback(async () => {
    setLoadingMapping(true);
    setErrorMapping(null);
    try {
      const data = await compAiApi.getControlPolicyMapping();
      setPolicyMapping(data);
    } catch (e) {
      setErrorMapping(e.message || 'Failed to load policy mapping');
    } finally {
      setLoadingMapping(false);
    }
  }, []);

  useEffect(() => {
    loadSystemDescription();
    loadPolicyMapping();
  }, [loadSystemDescription, loadPolicyMapping]);

  const handleFillTemplate = () => {
    setFilledText(fillTemplate(systemDescription.template, fillValues));
  };

  const handleCopy = async () => {
    const text = filledText || systemDescription.template;
    try {
      await navigator.clipboard.writeText(text);
      setCopySuccess(true);
      setTimeout(() => setCopySuccess(false), 2000);
    } catch {
      setCopySuccess(false);
    }
  };

  const handleDownloadMd = () => {
    const text = filledText || systemDescription.template;
    const blob = new Blob([text], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `system-description-${new Date().toISOString().slice(0, 10)}.md`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <PageTemplate
      title="Audit docs"
      subtitle="SOC 2 System Description and control–policy mapping"
    >
      <div className="comp-ai-container comp-ai-audit-docs-container">
        <div className="comp-ai-audit-docs-main">
          <section className="comp-ai-controls-section comp-ai-audit-docs-section">
            <h2>System Description (SOC 2)</h2>
            <p className="comp-ai-form-hint">
              Use the template below for your SOC 2 system description. Fill the placeholders and copy or download.
            </p>
            {loadingDesc && <div className="comp-ai-loading">Loading template...</div>}
            {errorDesc && <p className="comp-ai-error" role="alert">{errorDesc}</p>}
            {!loadingDesc && !errorDesc && (
              <>
                <div className="comp-ai-audit-docs-fill-form">
                  <label>
                    <span>Organisation name</span>
                    <input
                      type="text"
                      value={fillValues.organisation_name}
                      onChange={(e) => setFillValues((v) => ({ ...v, organisation_name: e.target.value }))}
                      placeholder="e.g. Acme Inc."
                      className="comp-ai-input"
                    />
                  </label>
                  <label>
                    <span>System / service in scope</span>
                    <input
                      type="text"
                      value={fillValues.system_name}
                      onChange={(e) => setFillValues((v) => ({ ...v, system_name: e.target.value }))}
                      placeholder="e.g. Customer portal and APIs"
                      className="comp-ai-input"
                    />
                  </label>
                  <label>
                    <span>As of date</span>
                    <input
                      type="date"
                      value={fillValues.as_of_date}
                      onChange={(e) => setFillValues((v) => ({ ...v, as_of_date: e.target.value }))}
                      className="comp-ai-input"
                    />
                  </label>
                  <button
                    type="button"
                    className="comp-ai-secondary-btn"
                    onClick={handleFillTemplate}
                  >
                    Fill template
                  </button>
                </div>
                <div className="comp-ai-audit-docs-actions">
                  <button
                    type="button"
                    className="comp-ai-secondary-btn"
                    onClick={handleCopy}
                  >
                    {copySuccess ? 'Copied!' : 'Copy to clipboard'}
                  </button>
                  <button
                    type="button"
                    className="comp-ai-secondary-btn"
                    onClick={handleDownloadMd}
                  >
                    Download .md
                  </button>
                </div>
                <pre className="comp-ai-audit-docs-template">
                  {(filledText || systemDescription.template)}
                </pre>
              </>
            )}
          </section>

          <section className="comp-ai-controls-section comp-ai-audit-docs-section">
            <h2>Control–policy mapping</h2>
            <p className="comp-ai-form-hint">
              Controls with linked policy or document evidence. Useful for auditors to see which policies support each control.
            </p>
            {loadingMapping && <div className="comp-ai-loading">Loading mapping...</div>}
            {errorMapping && <p className="comp-ai-error" role="alert">{errorMapping}</p>}
            {!loadingMapping && !errorMapping && (
              <>
                {policyMapping.mapping.length === 0 ? (
                  <p className="comp-ai-empty">No controls with policy/document evidence yet. Add evidence of type Document or Policy in Controls.</p>
                ) : (
                  <div className="comp-ai-audit-docs-mapping-table-wrap">
                    <table className="comp-ai-audit-docs-mapping-table">
                      <thead>
                        <tr>
                          <th>Control ID</th>
                          <th>Control name</th>
                          <th>Policies / documents</th>
                        </tr>
                      </thead>
                      <tbody>
                        {policyMapping.mapping.map((entry) => (
                          <tr key={entry.control_id}>
                            <td><code>{entry.internal_id}</code></td>
                            <td>{entry.name}</td>
                            <td>
                              {entry.policies.length === 0 ? (
                                <span className="comp-ai-meta">—</span>
                              ) : (
                                <ul className="comp-ai-audit-docs-policy-list">
                                  {entry.policies.map((p) => (
                                    <li key={p.id}>
                                      <span className="comp-ai-evidence-type">{p.type}</span>
                                      {p.source && ` ${p.source}`}
                                      {p.link_url && (
                                        <a href={p.link_url} target="_blank" rel="noopener noreferrer" className="comp-ai-evidence-link"> Link</a>
                                      )}
                                      {p.file_name && ` (${p.file_name})`}
                                    </li>
                                  ))}
                                </ul>
                              )}
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                )}
              </>
            )}
          </section>
        </div>
        <div className="comp-ai-sidebar">
          <div className="comp-ai-info-card">
            <h3>Audit docs</h3>
            <p>
              <strong>System Description</strong> — SOC 2 template; fill placeholders and export for auditors.
              <strong>Control–policy mapping</strong> — which policies/documents support each control.
            </p>
          </div>
          <div className="comp-ai-info-card">
            <h3>Quick links</h3>
            <ul>
              <li><Link to="/comp-ai">Comp AI chat</Link></li>
              <li><Link to="/comp-ai/controls">Controls &amp; Evidence</Link></li>
              <li><Link to="/comp-ai/scan">Scan documents</Link></li>
              <li><Link to="/comp-ai/history">Request history</Link></li>
              <li><Link to="/comp-ai/monitoring">Monitoring</Link></li>
            </ul>
          </div>
        </div>
      </div>
    </PageTemplate>
  );
}

export default CompAIAuditDocs;
