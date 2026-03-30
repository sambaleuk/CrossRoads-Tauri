import { useState, useEffect } from 'react';
import * as api from '../services/api';
import type { TrustScore, PerformanceProfile, AgentMemory } from '../models';

type IntelTab = 'trust' | 'memory' | 'models' | 'routing';

/**
 * Intelligence panel — shows how XRoads learns from your codebase.
 * Accessible via Cmd+Shift+L or the toolbar.
 */
export function IntelligencePanel({ onClose }: { onClose: () => void }) {
  const [tab, setTab] = useState<IntelTab>('trust');
  const [trustScores, setTrustScores] = useState<TrustScore[]>([]);
  const [profiles, setProfiles] = useState<PerformanceProfile[]>([]);
  const [memories, setMemories] = useState<AgentMemory[]>([]);
  const [memoryQuery, setMemoryQuery] = useState('');
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    (async () => {
      try {
        const [ts, pp, mems] = await Promise.all([
          api.fetchTrustScores(),
          api.fetchPerformanceProfiles(),
          api.searchMemories(''),
        ]);
        setTrustScores(ts);
        setProfiles(pp);
        if (mems.length > 0) setMemories(mems);
      } catch {}
      setLoading(false);
    })();
  }, []);

  const searchMemories = async () => {
    if (!memoryQuery.trim()) return;
    try {
      const results = await api.searchMemories(memoryQuery);
      setMemories(results);
    } catch {}
  };

  const tabs: { key: IntelTab; label: string }[] = [
    { key: 'trust', label: 'Trust Matrix' },
    { key: 'memory', label: 'Agent Memory' },
    { key: 'models', label: 'ML Models' },
    { key: 'routing', label: 'Cost Routing' },
  ];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" onClick={onClose}>
      <div className="w-[640px] max-h-[80vh] bg-gray-900 border border-gray-700 rounded-xl shadow-2xl flex flex-col xr-animate-scale-in"
        onClick={e => e.stopPropagation()}>

        {/* Header */}
        <div className="px-5 py-3 border-b border-gray-800 flex items-center justify-between">
          <div>
            <h2 className="text-sm font-bold font-mono text-gray-100">INTELLIGENCE</h2>
            <p className="text-[9px] font-mono text-gray-500 mt-0.5">How XRoads learns from your codebase</p>
          </div>
          <button onClick={onClose} className="text-gray-500 hover:text-gray-300 text-lg">×</button>
        </div>

        {/* Tabs */}
        <div className="flex border-b border-gray-800">
          {tabs.map(t => (
            <button key={t.key} onClick={() => setTab(t.key)}
              className={`flex-1 px-3 py-2 text-[10px] font-mono transition-colors ${
                tab === t.key ? 'text-neon-green border-b-2 border-neon-green' : 'text-gray-500 hover:text-gray-400'
              }`}>
              {t.label}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="flex-1 overflow-auto p-5">
          {loading ? (
            <div className="flex items-center justify-center h-32">
              <span className="text-[10px] font-mono text-gray-500 animate-pulse">Loading intelligence data...</span>
            </div>
          ) : (
            <>
              {tab === 'trust' && <TrustMatrixTab scores={trustScores} />}
              {tab === 'memory' && (
                <MemoryTab
                  memories={memories}
                  query={memoryQuery}
                  setQuery={setMemoryQuery}
                  onSearch={searchMemories}
                />
              )}
              {tab === 'models' && <ModelsTab profiles={profiles} />}
              {tab === 'routing' && <RoutingTab />}
            </>
          )}
        </div>
      </div>
    </div>
  );
}

// ── Trust Matrix ──

function TrustMatrixTab({ scores }: { scores: TrustScore[] }) {
  if (scores.length === 0) {
    return <Empty text="No trust data yet. Run orchestrations to build agent trust scores." />;
  }

  // Group by domain
  const domains = [...new Set(scores.map(s => s.domain))];
  const agents = [...new Set(scores.map(s => s.agentType))];

  return (
    <div className="space-y-4">
      <p className="text-[9px] font-mono text-gray-500">Per-agent reliability by code domain. Higher = more trusted for that type of work.</p>

      {/* Matrix table */}
      <div className="overflow-auto">
        <table className="w-full text-[10px] font-mono">
          <thead>
            <tr className="border-b border-gray-800">
              <th className="text-left py-1.5 text-gray-500 font-normal">Agent</th>
              {domains.map(d => (
                <th key={d} className="text-center py-1.5 text-gray-500 font-normal px-2">{d}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {agents.map(agent => (
              <tr key={agent} className="border-b border-gray-800/50">
                <td className="py-2 text-gray-300">
                  {agent === 'claude' ? '🧠' : agent === 'gemini' ? '✨' : '⌨️'} {agent}
                </td>
                {domains.map(domain => {
                  const score = scores.find(s => s.agentType === agent && s.domain === domain);
                  if (!score) return <td key={domain} className="text-center text-gray-700 px-2">—</td>;
                  const pct = Math.round(score.score * 100);
                  return (
                    <td key={domain} className="text-center px-2">
                      <span className={`font-bold ${pct >= 80 ? 'text-neon-green' : pct >= 50 ? 'text-amber-400' : 'text-red-400'}`}>
                        {pct}%
                      </span>
                      {score.autoMergeEnabled && <span className="text-[7px] text-neon-green ml-0.5">✓</span>}
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Legend */}
      <div className="flex gap-4 text-[8px] font-mono text-gray-600">
        <span><span className="text-neon-green">■</span> ≥80% trusted</span>
        <span><span className="text-amber-400">■</span> 50-79%</span>
        <span><span className="text-red-400">■</span> &lt;50%</span>
        <span><span className="text-neon-green">✓</span> auto-merge enabled</span>
      </div>
    </div>
  );
}

// ── Agent Memory ──

function MemoryTab({ memories, query, setQuery, onSearch }: {
  memories: AgentMemory[];
  query: string;
  setQuery: (q: string) => void;
  onSearch: () => void;
}) {
  return (
    <div className="space-y-3">
      <p className="text-[9px] font-mono text-gray-500">Persistent memories that survive across sessions. XRoads remembers what each agent is good (and bad) at.</p>

      {/* Search */}
      <div className="flex gap-2">
        <input
          type="text" value={query} onChange={e => setQuery(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && onSearch()}
          placeholder="Search memories (e.g. 'typescript', 'conflict')..."
          className="flex-1 bg-gray-800 border border-gray-700 rounded px-3 py-1.5 text-[11px] font-mono text-gray-200 placeholder:text-gray-600 focus:border-neon-green/40 focus:outline-none"
        />
        <button onClick={onSearch} className="px-3 py-1.5 text-[10px] font-mono bg-neon-green/20 text-neon-green border border-neon-green/30 rounded hover:bg-neon-green/30">
          Search
        </button>
      </div>

      {/* Results */}
      {memories.length === 0 ? (
        <div className="text-[10px] font-mono text-gray-600 text-center py-8">
          {query ? 'No memories found for this query.' : 'Search for memories or run orchestrations to build them.'}
        </div>
      ) : (
        <div className="space-y-2">
          {memories.map(m => (
            <div key={m.id} className="p-2.5 rounded bg-gray-800/40 border border-gray-800 space-y-1">
              <div className="flex items-center gap-2">
                <span className="text-[9px] font-bold font-mono text-neon-blue">{m.agentType}</span>
                <span className="text-[8px] font-mono text-gray-600">{m.domain}</span>
                <span className="text-[7px] font-mono text-gray-700 bg-gray-800 px-1 rounded">{m.memoryType}</span>
                <div className="flex-1" />
                <span className="text-[8px] font-mono text-gray-600">conf: {Math.round(m.confidence * 100)}%</span>
              </div>
              <div className="text-[10px] font-mono text-gray-300">{m.content}</div>
              <div className="text-[8px] font-mono text-gray-700">accessed {m.accessCount}x</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ── ML Models ──

function ModelsTab({ profiles }: { profiles: PerformanceProfile[] }) {
  const categories = [...new Set(profiles.map(p => p.taskCategory))];
  const totalSamples = profiles.reduce((sum, p) => sum + p.totalExecutions, 0);

  return (
    <div className="space-y-4">
      <p className="text-[9px] font-mono text-gray-500">3 ML models train locally after each orchestration. No data leaves your machine.</p>

      {/* Model cards */}
      <div className="grid grid-cols-3 gap-3">
        <ModelCard name="TimeEstimator" type="LinearRegression" desc="Story completion time" trained={totalSamples >= 10} samples={totalSamples} />
        <ModelCard name="Categorizer" type="NaiveBayes" desc="Task domain classification" trained={totalSamples >= 10} samples={totalSamples} />
        <ModelCard name="ConflictPredictor" type="DecisionTree" desc="Merge conflict risk" trained={totalSamples >= 10} samples={totalSamples} />
      </div>

      {/* Performance profiles */}
      {profiles.length > 0 && (
        <div>
          <h3 className="text-[9px] font-bold font-mono text-gray-500 uppercase mb-2">Performance Profiles ({categories.length} categories)</h3>
          <div className="space-y-1.5">
            {profiles.map(p => (
              <div key={p.id} className="flex items-center gap-3 text-[10px] font-mono">
                <span className="text-gray-300 w-20 truncate">{p.agentType}</span>
                <span className="text-gray-500 w-24 truncate">{p.taskCategory}</span>
                <div className="flex-1 h-1 bg-gray-800 rounded-full overflow-hidden">
                  <div className="h-full bg-neon-green rounded-full" style={{ width: `${p.successRate * 100}%` }} />
                </div>
                <span className="text-gray-400 w-10 text-right">{Math.round(p.successRate * 100)}%</span>
                <span className="text-gray-600 w-8 text-right">{p.totalExecutions}x</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function ModelCard({ name, type, desc, trained, samples }: { name: string; type: string; desc: string; trained: boolean; samples: number }) {
  return (
    <div className={`p-3 rounded border ${trained ? 'border-neon-green/30 bg-neon-green/5' : 'border-gray-700 bg-gray-800/30'}`}>
      <div className="text-[10px] font-bold font-mono text-gray-200">{name}</div>
      <div className="text-[8px] font-mono text-neon-purple mt-0.5">{type}</div>
      <div className="text-[9px] font-mono text-gray-500 mt-1">{desc}</div>
      <div className={`text-[8px] font-mono mt-2 ${trained ? 'text-neon-green' : 'text-gray-600'}`}>
        {trained ? `✅ Trained (${samples} samples)` : `⏳ Need ${10 - samples} more samples`}
      </div>
    </div>
  );
}

// ── Cost Routing ──

function RoutingTab() {
  return (
    <div className="space-y-4">
      <p className="text-[9px] font-mono text-gray-500">XRoads automatically selects the optimal model based on budget pressure and task complexity.</p>

      <div className="space-y-3">
        <RoutingRow model="opus" provider="Anthropic" cost="$15/$75 per M" capability={1.0} use="Complex/critical tasks, healthy budget" />
        <RoutingRow model="sonnet" provider="Anthropic" cost="$3/$15 per M" capability={0.8} use="Moderate tasks, any budget" />
        <RoutingRow model="haiku" provider="Anthropic" cost="$0.25/$1.25 per M" capability={0.5} use="Simple tasks, tight budget" />
        <RoutingRow model="gpt-4o" provider="OpenAI" cost="$2.5/$10 per M" capability={0.85} use="Balanced alternative" />
        <RoutingRow model="o3" provider="OpenAI" cost="$10/$40 per M" capability={0.95} use="Complex reasoning" />
        <RoutingRow model="gemini-2" provider="Google" cost="$0.50/$1.50 per M" capability={0.7} use="Cost-efficient option" />
      </div>

      <div className="p-3 rounded bg-gray-800/30 border border-gray-800">
        <div className="text-[9px] font-bold font-mono text-gray-500 uppercase mb-1">How routing works</div>
        <div className="text-[10px] font-mono text-gray-400 space-y-1">
          <p>Score = capability × {'{capWeight}'} + costEfficiency × {'{costWeight}'}</p>
          <p>Budget healthy → capWeight=0.8, costWeight=0.2 (best model)</p>
          <p>Budget critical → capWeight=0.1, costWeight=0.9 (cheapest viable)</p>
        </div>
      </div>
    </div>
  );
}

function RoutingRow({ model, provider, cost, capability, use }: { model: string; provider: string; cost: string; capability: number; use: string }) {
  return (
    <div className="flex items-center gap-3 p-2 rounded bg-gray-800/30">
      <div className="w-20">
        <div className="text-[10px] font-bold font-mono text-gray-200">{model}</div>
        <div className="text-[8px] font-mono text-gray-600">{provider}</div>
      </div>
      <div className="flex-1">
        <div className="h-1 bg-gray-800 rounded-full overflow-hidden">
          <div className="h-full bg-neon-blue rounded-full" style={{ width: `${capability * 100}%` }} />
        </div>
      </div>
      <div className="w-28 text-right">
        <div className="text-[9px] font-mono text-gray-400">{cost}</div>
        <div className="text-[8px] font-mono text-gray-600">{use}</div>
      </div>
    </div>
  );
}

// ── Empty state ──

function Empty({ text }: { text: string }) {
  return (
    <div className="flex items-center justify-center h-32">
      <p className="text-[10px] font-mono text-gray-600 text-center">{text}</p>
    </div>
  );
}
