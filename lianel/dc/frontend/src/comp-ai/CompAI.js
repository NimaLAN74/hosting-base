import React, { useState, useEffect, useRef } from 'react';
import { Link } from 'react-router-dom';
import { compAiApi } from './compAiApi';
import PageTemplate from '../PageTemplate';
import './CompAI.css';

function CompAI() {
  const [prompt, setPrompt] = useState('');
  const [framework, setFramework] = useState('');
  const [frameworks, setFrameworks] = useState([]);
  const [conversation, setConversation] = useState([]); // [{ userMessage, assistantResponse }, ...]
  const [response, setResponse] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [processingTime, setProcessingTime] = useState(null);
  const threadEndRef = useRef(null);

  useEffect(() => {
    compAiApi.getFrameworks().then((data) => {
      setFrameworks(data.frameworks || []);
    }).catch(() => setFrameworks([]));
  }, []);

  useEffect(() => {
    threadEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [conversation, response]);

  const handleNewConversation = () => {
    setConversation([]);
    setResponse(null);
    setError(null);
    setProcessingTime(null);
    setPrompt('');
  };

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
    const userMessage = prompt.trim();

    // Build messages for API: previous turns only (backend appends current prompt as last user message)
    const messagesForApi =
      conversation.length > 0
        ? conversation.flatMap(({ userMessage: u, assistantResponse: a }) => [
            { role: 'user', content: u },
            { role: 'assistant', content: a },
          ])
        : null;

    try {
      const result = await compAiApi.processRequest(
        userMessage,
        framework || null,
        messagesForApi
      );
      const endTime = Date.now();
      setProcessingTime(endTime - startTime);
      setResponse(result);
      setConversation((prev) => [
        ...prev,
        { userMessage, assistantResponse: result.response || result.text || '' },
      ]);
      setPrompt('');
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
          {(conversation.length > 0 || response) && (
            <div className="comp-ai-conversation-section">
              <div className="comp-ai-conversation-header">
                <h2>Conversation</h2>
                <button
                  type="button"
                  onClick={handleNewConversation}
                  disabled={loading}
                  className="comp-ai-new-conversation-btn"
                >
                  New conversation
                </button>
              </div>
              <div className="comp-ai-thread">
                {conversation.map((turn, i) => (
                  <React.Fragment key={i}>
                    <div className="comp-ai-message comp-ai-message-user">
                      <span className="comp-ai-message-role">You</span>
                      <div className="comp-ai-message-content">{turn.userMessage}</div>
                    </div>
                    <div className="comp-ai-message comp-ai-message-assistant">
                      <span className="comp-ai-message-role">Comp-AI</span>
                      <div className="comp-ai-message-content">{turn.assistantResponse}</div>
                    </div>
                  </React.Fragment>
                ))}
                <div ref={threadEndRef} />
              </div>
            </div>
          )}

          <div className="comp-ai-request-section">
            <h2>{conversation.length > 0 ? 'Follow-up' : 'Submit Request'}</h2>
            <form onSubmit={handleSubmit} className="comp-ai-form">
              <div className="form-group">
                <label htmlFor="framework">Compliance framework (optional):</label>
                <select
                  id="framework"
                  value={framework}
                  onChange={(e) => setFramework(e.target.value)}
                  disabled={loading}
                  className="comp-ai-select"
                >
                  <option value="">General (no framework)</option>
                  {(frameworks || []).map((f) => (
                    <option key={f.id} value={f.id}>{f.name}</option>
                  ))}
                </select>
              </div>
              <div className="form-group">
                <label htmlFor="prompt">Enter your prompt or question:</label>
                <textarea
                  id="prompt"
                  value={prompt}
                  onChange={(e) => setPrompt(e.target.value)}
                  placeholder={
                    conversation.length > 0
                      ? 'Ask a follow-up...'
                      : 'e.g., What does CC6.1 require for logical access?'
                  }
                  rows={4}
                  disabled={loading}
                  className="comp-ai-textarea"
                />
              </div>
              <button
                type="submit"
                disabled={loading || !prompt.trim()}
                className="comp-ai-submit-btn"
              >
                {loading ? 'Processing...' : conversation.length > 0 ? 'Send' : 'Submit Request'}
              </button>
            </form>

            {error && (
              <div className="comp-ai-error">
                <strong>Error:</strong> {error}
              </div>
            )}

            {processingTime != null && (
              <div className="comp-ai-meta">
                Processing time: {processingTime}ms
              </div>
            )}
          </div>
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
              <li><Link to="/comp-ai/controls">Controls &amp; Evidence</Link> — map controls and attach evidence</li>
            </ul>
          </div>
        </div>
      </div>
    </PageTemplate>
  );
}

export default CompAI;
