import React, { useState, useEffect } from 'react';
import { compAiApi } from './compAiApi';
import PageTemplate from '../PageTemplate';
import './CompAI.css';

function CompAIHistory() {
  const [history, setHistory] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [total, setTotal] = useState(0);
  const [offset, setOffset] = useState(0);
  const [limit] = useState(50);

  useEffect(() => {
    loadHistory();
  }, [offset]);

  const loadHistory = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await compAiApi.getRequestHistory({ limit, offset });
      setHistory(result.data || []);
      setTotal(result.total || 0);
    } catch (err) {
      setError(err.message || 'Failed to load request history');
    } finally {
      setLoading(false);
    }
  };

  const formatDate = (dateString) => {
    if (!dateString) return 'N/A';
    try {
      const date = new Date(dateString);
      return date.toLocaleString();
    } catch {
      return dateString;
    }
  };

  const truncateText = (text, maxLength = 100) => {
    if (!text) return 'N/A';
    if (text.length <= maxLength) return text;
    return text.substring(0, maxLength) + '...';
  };

  return (
    <PageTemplate title="Comp AI Request History" subtitle="View your past requests and responses">
      <div className="comp-ai-container">
        <div className="comp-ai-history">
          {loading && offset === 0 && (
            <div className="comp-ai-loading">Loading history...</div>
          )}

          {error && (
            <div className="comp-ai-error">
              <strong>Error:</strong> {error}
            </div>
          )}

          {!loading && !error && history.length === 0 && (
            <div className="comp-ai-empty">
              <p>No request history found.</p>
              <p>Start by submitting a request from the main Comp AI page.</p>
            </div>
          )}

          {!loading && history.length > 0 && (
            <>
              <div className="comp-ai-history-header">
                <h3>Total Requests: {total}</h3>
                <div className="comp-ai-pagination">
                  <button
                    onClick={() => setOffset(Math.max(0, offset - limit))}
                    disabled={offset === 0}
                    className="comp-ai-pagination-btn"
                  >
                    Previous
                  </button>
                  <span>
                    Showing {offset + 1}-{Math.min(offset + limit, total)} of {total}
                  </span>
                  <button
                    onClick={() => setOffset(offset + limit)}
                    disabled={offset + limit >= total}
                    className="comp-ai-pagination-btn"
                  >
                    Next
                  </button>
                </div>
              </div>

              <div className="comp-ai-history-list">
                {history.map((item) => (
                  <div key={item.id} className="comp-ai-history-item">
                    <div className="comp-ai-history-item-header">
                      <span className="comp-ai-history-date">
                        {formatDate(item.created_at)}
                      </span>
                      {item.model_used && (
                        <span className="comp-ai-history-model">{item.model_used}</span>
                      )}
                      {item.tokens_used && (
                        <span className="comp-ai-history-tokens">
                          {item.tokens_used} tokens
                        </span>
                      )}
                    </div>
                    <div className="comp-ai-history-prompt">
                      <strong>Prompt:</strong>
                      <p>{item.prompt || item.request_text}</p>
                    </div>
                    {item.response && (
                      <div className="comp-ai-history-response">
                        <strong>Response:</strong>
                        <p>{truncateText(item.response || item.response_text, 200)}</p>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </>
          )}
        </div>
      </div>
    </PageTemplate>
  );
}

export default CompAIHistory;
