import { useState, useEffect } from 'react';
import * as api from '../services/api';
import type { UsageSummary } from '../models';

interface Props {
  sessionId: string;
  sessionCost: UsageSummary;
  slots: Array<{ agentType: string; status: string; currentTask?: string }>;
  startedAt: string;
  onClose: () => void;
}

/**
 * Review overlay — appears when orchestration completes.
 * Shows: story results, agent performance, cost, learnings.
 */
export function ReviewOverlay({ sessionId, sessionCost, slots, startedAt, onClose }: Props) {
  const [retro, setRetro] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    (async () => {
      try {
        const r = await api.generateRetro(sessionId);
        setRetro(r);
      } catch { setRetro(null); }
      setLoading(false);
    })();
  }, [sessionId]);

  const done = slots.filter(s => s.status === 'done').length;
  const errors = slots.filter(s => s.status === 'error').length;
  const total = slots.length;

  const costStr = sessionCost.totalCostCents >= 100
    ? `$${(sessionCost.totalCostCents / 100).toFixed(2)}`
    : `${sessionCost.totalCostCents}¢`;

  const elapsed = Math.round((Date.now() - new Date(startedAt).getTime()) / 60000);

  // Agent breakdown
  const agentMap = new Map<string, { count: number; done: number }>();
  for (const slot of slots) {
    const entry = agentMap.get(slot.agentType) ?? { count: 0, done: 0 };
    entry.count++;
    if (slot.status === 'done') entry.done++;
    agentMap.set(slot.agentType, entry);
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm" onClick={onClose}>
      <div className="w-[560px] max-h-[80vh] bg-gray-900 border border-gray-700 rounded-xl shadow-2xl flex flex-col xr-animate-scale-in"
        onClick={e => e.stopPropagation()}>

        {/* Header */}
        <div className="px-5 py-4 border-b border-gray-800">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-sm font-bold font-mono text-gray-100">ORCHESTRATION COMPLETE</h2>
              <div className="flex items-center gap-3 mt-1 text-[10px] font-mono text-gray-400">
                <span>{elapsed}min</span>
                <span className="text-gray-700">|</span>
                <span className={sessionCost.totalCostCents > 1000 ? 'text-amber-400' : 'text-neon-green'}>{costStr}</span>
                <span className="text-gray-700">|</span>
                <span>{done}/{total} stories</span>
              </div>
            </div>
            <button onClick={onClose} className="text-gray-500 hover:text-gray-300 text-lg">×</button>
          </div>
        </div>

        {/* Progress summary */}
        <div className="px-5 py-3 border-b border-gray-800/50">
          <div className="h-2 bg-gray-800 rounded-full overflow-hidden">
            <div className="h-full flex">
              <div className="bg-neon-green transition-all" style={{ width: `${total > 0 ? (done / total) * 100 : 0}%` }} />
              {errors > 0 && <div className="bg-red-500" style={{ width: `${(errors / total) * 100}%` }} />}
            </div>
          </div>
          <div className="flex justify-between mt-1 text-[9px] font-mono text-gray-500">
            <span>{done} completed</span>
            {errors > 0 && <span className="text-red-400">{errors} failed</span>}
            <span>{total - done - errors} remaining</span>
          </div>
        </div>

        {/* Scrollable content */}
        <div className="flex-1 overflow-auto px-5 py-4 space-y-5">

          {/* Agent Performance */}
          <Section title="AGENT PERFORMANCE">
            <div className="space-y-2">
              {Array.from(agentMap.entries()).map(([agent, data]) => (
                <div key={agent} className="flex items-center gap-3">
                  <span className="text-[11px] font-mono text-gray-300 w-16">
                    {agent === 'claude' ? '🧠' : agent === 'gemini' ? '✨' : '⌨️'} {agent}
                  </span>
                  <div className="flex-1 h-1.5 bg-gray-800 rounded-full overflow-hidden">
                    <div className="h-full bg-neon-green rounded-full" style={{ width: `${data.count > 0 ? (data.done / data.count) * 100 : 0}%` }} />
                  </div>
                  <span className="text-[10px] font-mono text-gray-400 w-12 text-right">{data.done}/{data.count}</span>
                </div>
              ))}
            </div>
          </Section>

          {/* Cost Breakdown */}
          <Section title="COST">
            <div className="grid grid-cols-3 gap-3">
              <MetricBox label="Total" value={costStr} />
              <MetricBox label="API Calls" value={`${sessionCost.eventCount}`} />
              <MetricBox label="Tokens" value={formatTokens(sessionCost.totalInputTokens + sessionCost.totalOutputTokens)} />
            </div>
          </Section>

          {/* Slot Results */}
          <Section title="SLOTS">
            <div className="space-y-1">
              {slots.map((slot, i) => (
                <div key={i} className="flex items-center gap-2 text-[10px] font-mono py-1">
                  <span className={`w-2 h-2 rounded-full ${slot.status === 'done' ? 'bg-neon-green' : slot.status === 'error' ? 'bg-red-400' : 'bg-gray-600'}`} />
                  <span className="text-gray-400">#{i + 1}</span>
                  <span className="text-gray-300">{slot.agentType}</span>
                  <span className="text-gray-500 truncate flex-1">{slot.currentTask ?? ''}</span>
                  <span className={slot.status === 'done' ? 'text-neon-green' : slot.status === 'error' ? 'text-red-400' : 'text-gray-500'}>
                    {slot.status}
                  </span>
                </div>
              ))}
            </div>
          </Section>

          {/* Auto-generated Retro */}
          <Section title="LEARNINGS">
            {loading ? (
              <div className="text-[10px] font-mono text-gray-500 animate-pulse">Generating retrospective...</div>
            ) : retro ? (
              <pre className="text-[10px] font-mono text-gray-400 whitespace-pre-wrap leading-relaxed max-h-48 overflow-auto">{retro}</pre>
            ) : (
              <div className="text-[10px] font-mono text-gray-600">Not enough data for retrospective yet.</div>
            )}
          </Section>
        </div>

        {/* Footer */}
        <div className="px-5 py-3 border-t border-gray-800 flex justify-end gap-3">
          <button onClick={onClose}
            className="px-4 py-1.5 text-[10px] font-mono text-gray-400 hover:text-gray-200 transition-colors">
            Close
          </button>
          <button onClick={onClose}
            className="px-4 py-1.5 text-[10px] font-mono bg-neon-green/20 text-neon-green border border-neon-green/30 rounded hover:bg-neon-green/30 transition-colors">
            View Branch
          </button>
        </div>
      </div>
    </div>
  );
}

// ── Helpers ──

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <h3 className="text-[9px] font-bold font-mono text-gray-500 uppercase tracking-wider mb-2">{title}</h3>
      {children}
    </div>
  );
}

function MetricBox({ label, value }: { label: string; value: string }) {
  return (
    <div className="p-2 rounded bg-gray-800/40 text-center">
      <div className="text-[8px] font-mono text-gray-500 uppercase">{label}</div>
      <div className="text-[13px] font-bold font-mono text-gray-200">{value}</div>
    </div>
  );
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return `${n}`;
}
