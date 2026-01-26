import React, { useState } from 'react';
import { compAiApi } from './compAiApi';
import PageTemplate from '../PageTemplate';
import './CompAI.css';

function CompAI() {
  const [prompt, setPrompt] = useState('');
  const [response, setResponse] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [processingTime, setProcessingTime] = useState(null);

  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!prompt.trim()) {
      setError('Please enter a prompt');
      return;
    }

    setLoading(true);
    setError(null);
    setResponse(null);
    setProcessingTime(null);

    const startTime = Date.now();

    try {
      const result = await compAiApi.processRequest(prompt);
      const endTime = Date.now();
      setProcessingTime(endTime - startTime);
      setResponse(result);
    } catch (err) {
      setError(err.message || 'Failed to process request');
    } finally {
      setLoading(false);
    }
  };

  return (
    <PageTemplate title="Comp AI Service" subtitle="AI-powered analysis and insights">
      <div className="comp-ai-container">
        <div className="comp-ai-main">
          <div className="comp-ai-request-section">
            <h2>Submit Request</h2>
            <form onSubmit={handleSubmit} className="comp-ai-form">
              <div className="form-group">
                <label htmlFor="prompt">Enter your prompt or question:</label>
                <textarea
                  id="prompt"
                  value={prompt}
                  onChange={(e) => setPrompt(e.target.value)}
                  placeholder="e.g., Analyze the energy consumption trends for Sweden in 2023..."
                  rows={6}
                  disabled={loading}
                  className="comp-ai-textarea"
                />
              </div>
              <button
                type="submit"
                disabled={loading || !prompt.trim()}
                className="comp-ai-submit-btn"
              >
                {loading ? 'Processing...' : 'Submit Request'}
              </button>
            </form>

            {error && (
              <div className="comp-ai-error">
                <strong>Error:</strong> {error}
              </div>
            )}

            {processingTime && (
              <div className="comp-ai-meta">
                Processing time: {processingTime}ms
              </div>
            )}
          </div>

          {response && (
            <div className="comp-ai-response-section">
              <h2>Response</h2>
              <div className="comp-ai-response">
                <div className="comp-ai-response-content">
                  {response.response || response.text || JSON.stringify(response, null, 2)}
                </div>
                {response.model_used && (
                  <div className="comp-ai-response-meta">
                    <span>Model: {response.model_used}</span>
                    {response.tokens_used && <span>Tokens: {response.tokens_used}</span>}
                  </div>
                )}
              </div>
            </div>
          )}
        </div>

        <div className="comp-ai-sidebar">
          <div className="comp-ai-info-card">
            <h3>About Comp AI</h3>
            <p>
              Comp AI provides AI-powered analysis and insights for your data queries.
              Submit prompts to get intelligent responses and analysis.
            </p>
          </div>

          <div className="comp-ai-info-card">
            <h3>Usage Tips</h3>
            <ul>
              <li>Be specific in your prompts</li>
              <li>Include relevant context</li>
              <li>Ask clear, focused questions</li>
              <li>Check request history for past queries</li>
            </ul>
          </div>
        </div>
      </div>
    </PageTemplate>
  );
}

export default CompAI;
