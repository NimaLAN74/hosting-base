import React, { useCallback, useEffect, useMemo, useState } from 'react';
import PageTemplate from '../PageTemplate';
import './SimulatorPage.css';

const RUNS_URL = '/api/v1/stock-service/sim/runs';

function fmtTs(ts) {
  if (!ts) return '—';
  const d = new Date(Number(ts) * 1000);
  if (Number.isNaN(d.getTime())) return '—';
  return d.toLocaleString('sv-SE', { hour12: false });
}

export default function SimulatorPage() {
  const [runs, setRuns] = useState([]);
  const [runsLoading, setRunsLoading] = useState(true);
  const [runsError, setRunsError] = useState('');
  const [selectedRunId, setSelectedRunId] = useState('');
  const [runStatus, setRunStatus] = useState(null);
  const [timeline, setTimeline] = useState([]);
  const [exchangeFilter, setExchangeFilter] = useState('ALL');
  const [biasFindings, setBiasFindings] = useState([]);
  const [explainData, setExplainData] = useState(null);
  const [explainError, setExplainError] = useState('');
  const [form, setForm] = useState({
    days: 7,
    top: 16,
    quantile: 0.2,
    short_enabled: true,
    initial_capital_usd: 100,
    replay_delay_ms: 250,
    reinvest_profit: true,
  });
  const [startLoading, setStartLoading] = useState(false);
  const [startMsg, setStartMsg] = useState('');

  const loadRuns = useCallback(() => {
    setRunsLoading(true);
    setRunsError('');
    fetch(`${RUNS_URL}?limit=30`, { credentials: 'include', headers: { Accept: 'application/json' } })
      .then(async (r) => {
        const j = await r.json().catch(() => null);
        if (!r.ok) throw new Error(j?.error || `Failed (${r.status})`);
        return j;
      })
      .then((j) => {
        const nextRuns = Array.isArray(j?.runs) ? j.runs : [];
        setRuns(nextRuns);
        if (!selectedRunId && nextRuns.length > 0) {
          setSelectedRunId(nextRuns[0].run_id);
        }
      })
      .catch((e) => setRunsError(String(e?.message || e)))
      .finally(() => setRunsLoading(false));
  }, [selectedRunId]);

  const loadSelectedRun = useCallback(() => {
    if (!selectedRunId) return;
    const statusUrl = `/api/v1/stock-service/sim/runs/${encodeURIComponent(selectedRunId)}/status`;
    const timelineUrl = exchangeFilter === 'ALL'
      ? `/api/v1/stock-service/sim/runs/${encodeURIComponent(selectedRunId)}/timeline?limit=400`
      : `/api/v1/stock-service/sim/runs/${encodeURIComponent(selectedRunId)}/exchanges/${encodeURIComponent(exchangeFilter)}?limit=400`;
    const biasUrl = `/api/v1/stock-service/sim/runs/${encodeURIComponent(selectedRunId)}/bias-report`;

    fetch(statusUrl, { credentials: 'include', headers: { Accept: 'application/json' } })
      .then((r) => r.json())
      .then((j) => setRunStatus(j))
      .catch(() => setRunStatus(null));

    fetch(timelineUrl, { credentials: 'include', headers: { Accept: 'application/json' } })
      .then((r) => r.json())
      .then((j) => setTimeline(Array.isArray(j?.events) ? j.events : []))
      .catch(() => setTimeline([]));

    fetch(biasUrl, { credentials: 'include', headers: { Accept: 'application/json' } })
      .then((r) => r.json())
      .then((j) => setBiasFindings(Array.isArray(j?.findings) ? j.findings : []))
      .catch(() => setBiasFindings([]));
  }, [selectedRunId, exchangeFilter]);

  useEffect(() => {
    loadRuns();
  }, [loadRuns]);

  useEffect(() => {
    loadSelectedRun();
    const id = setInterval(loadSelectedRun, 2500);
    return () => clearInterval(id);
  }, [loadSelectedRun]);

  const exchanges = useMemo(() => {
    const set = new Set();
    runs.forEach((r) => (r.exchanges || []).forEach((x) => set.add(x)));
    return ['ALL', ...Array.from(set)];
  }, [runs]);

  const decisionEvents = useMemo(
    () => timeline.filter((e) => e?.kind === 'DecisionCreated' && e?.payload?.decision_id),
    [timeline]
  );

  const handleStart = async () => {
    setStartLoading(true);
    setStartMsg('');
    try {
      const r = await fetch(RUNS_URL, {
        method: 'POST',
        credentials: 'include',
        headers: {
          'Content-Type': 'application/json',
          Accept: 'application/json',
        },
        body: JSON.stringify(form),
      });
      const j = await r.json().catch(() => null);
      if (!r.ok) throw new Error(j?.error || `Failed (${r.status})`);
      setStartMsg(`Started run ${j.run_id}`);
      setSelectedRunId(j.run_id);
      loadRuns();
    } catch (e) {
      setStartMsg(`Start failed: ${String(e?.message || e)}`);
    } finally {
      setStartLoading(false);
    }
  };

  const loadExplain = async (decisionId) => {
    setExplainData(null);
    setExplainError('');
    if (!selectedRunId || !decisionId) return;
    try {
      const url = `/api/v1/stock-service/sim/runs/${encodeURIComponent(selectedRunId)}/decision/${encodeURIComponent(decisionId)}/explain`;
      const r = await fetch(url, { credentials: 'include', headers: { Accept: 'application/json' } });
      const j = await r.json().catch(() => null);
      if (!r.ok) throw new Error(j?.error || `Failed (${r.status})`);
      setExplainData(j);
    } catch (e) {
      setExplainError(String(e?.message || e));
    }
  };

  return (
    <PageTemplate title="Exchange Replay Simulator" subtitle="Multi-exchange replay with full decision traceability">
      <div className="sim-grid">
        <section className="sim-card">
          <h3>Start simulation</h3>
          <p className="sim-note">Starts with USD 100 by default and reinvests realized PnL.</p>
          <div className="sim-form-grid">
            <label>Days
              <input type="number" min="5" max="30" value={form.days} onChange={(e) => setForm((f) => ({ ...f, days: Number(e.target.value || 7) }))} />
            </label>
            <label>Top symbols
              <input type="number" min="6" max="40" value={form.top} onChange={(e) => setForm((f) => ({ ...f, top: Number(e.target.value || 16) }))} />
            </label>
            <label>Quantile
              <input type="number" step="0.01" min="0.05" max="0.45" value={form.quantile} onChange={(e) => setForm((f) => ({ ...f, quantile: Number(e.target.value || 0.2) }))} />
            </label>
            <label>Initial capital (USD)
              <input type="number" min="25" step="1" value={form.initial_capital_usd} onChange={(e) => setForm((f) => ({ ...f, initial_capital_usd: Number(e.target.value || 100) }))} />
            </label>
            <label>Replay delay ms
              <input type="number" min="0" step="50" value={form.replay_delay_ms} onChange={(e) => setForm((f) => ({ ...f, replay_delay_ms: Number(e.target.value || 250) }))} />
            </label>
            <label className="sim-check">
              <input type="checkbox" checked={form.short_enabled} onChange={(e) => setForm((f) => ({ ...f, short_enabled: e.target.checked }))} />
              Allow short side
            </label>
            <label className="sim-check">
              <input type="checkbox" checked={form.reinvest_profit} onChange={(e) => setForm((f) => ({ ...f, reinvest_profit: e.target.checked }))} />
              Reinvest profit/loss
            </label>
          </div>
          <button className="sim-btn" disabled={startLoading} onClick={handleStart}>
            {startLoading ? 'Starting…' : 'Start run'}
          </button>
          {startMsg && <p className="sim-note">{startMsg}</p>}
        </section>

        <section className="sim-card">
          <h3>Runs</h3>
          {runsLoading && <p className="sim-note">Loading runs…</p>}
          {runsError && <p className="sim-error">{runsError}</p>}
          {!runsLoading && runs.length === 0 && <p className="sim-note">No simulator runs yet.</p>}
          <div className="sim-runs-list">
            {runs.map((r) => (
              <button
                key={r.run_id}
                className={`sim-run-row ${selectedRunId === r.run_id ? 'active' : ''}`}
                onClick={() => setSelectedRunId(r.run_id)}
              >
                <span>{r.run_id}</span>
                <span>{r.status}</span>
                <span>{fmtTs(r.created_at_ts)}</span>
              </button>
            ))}
          </div>
        </section>
      </div>

      <section className="sim-card">
        <div className="sim-row-head">
          <h3>Realtime run view</h3>
          <select value={exchangeFilter} onChange={(e) => setExchangeFilter(e.target.value)}>
            {exchanges.map((x) => (
              <option key={x} value={x}>{x}</option>
            ))}
          </select>
        </div>
        {runStatus ? (
          <div className="sim-status-grid">
            <div>Status: <b>{runStatus.status}</b></div>
            <div>Initial: <b>${Number(runStatus.initial_capital_usd || 0).toFixed(2)}</b></div>
            <div>Ending: <b>{runStatus.ending_equity_usd != null ? `$${Number(runStatus.ending_equity_usd).toFixed(2)}` : '—'}</b></div>
            <div>PnL: <b>{runStatus.pnl_usd != null ? `$${Number(runStatus.pnl_usd).toFixed(2)}` : '—'}</b></div>
            <div>Symbols: <b>{Number(runStatus.symbols_count || 0)}</b></div>
            <div>Exchanges: <b>{(runStatus.exchanges || []).join(', ') || '—'}</b></div>
          </div>
        ) : (
          <p className="sim-note">Pick a run to inspect realtime timeline.</p>
        )}
        <div className="sim-timeline">
          {timeline.map((e) => (
            <div key={e.event_id} className="sim-event-row">
              <span>{fmtTs(e.ts)}</span>
              <span>{e.kind}</span>
              <span>{e.exchange || '—'}</span>
            </div>
          ))}
        </div>
      </section>

      <div className="sim-grid">
        <section className="sim-card">
          <h3>Bias and missing-data findings</h3>
          {biasFindings.length === 0 && <p className="sim-note">No findings in current run.</p>}
          {biasFindings.map((f, i) => (
            <div key={`${f.code}-${i}`} className="sim-finding">
              <div><b>{f.severity}</b> · {f.code}</div>
              <div>{f.message}</div>
            </div>
          ))}
        </section>

        <section className="sim-card">
          <h3>Decision explain</h3>
          <div className="sim-decision-list">
            {decisionEvents.slice(0, 40).map((e) => (
              <button
                key={e.event_id}
                className="sim-run-row"
                onClick={() => loadExplain(e.payload.decision_id)}
              >
                <span>{e.payload.symbol}</span>
                <span>{e.payload.side}</span>
                <span>{e.payload.decision_id}</span>
              </button>
            ))}
          </div>
          {explainError && <p className="sim-error">{explainError}</p>}
          {explainData && (
            <pre className="sim-json">{JSON.stringify(explainData, null, 2)}</pre>
          )}
        </section>
      </div>
    </PageTemplate>
  );
}

