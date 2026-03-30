import { useState, useEffect, useCallback } from 'react';
import { useAppStore } from '../stores/appStore';
import * as api from '../services/api';
import type { CockpitOrchestrationPlan, AdaptationAction } from '../models';

export function CockpitBrainTab() {
  const { projectPath, session } = useAppStore();
  const [plan, setPlan] = useState<CockpitOrchestrationPlan | null>(null);
  const [adaptations, setAdaptations] = useState<AdaptationAction[]>([]);
  const [expandedCategories, setExpandedCategories] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(false);

  const fetchPlan = useCallback(async () => {
    if (!projectPath) return;
    try {
      const p = await api.getCockpitPlan(projectPath);
      if (p) setPlan(p);
    } catch { /* plan not generated yet */ }
  }, [projectPath]);

  const fetchAdaptations = useCallback(async () => {
    if (!session?.id) return;
    try {
      const actions = await api.getAdaptationActions(session.id);
      setAdaptations(actions);
    } catch { /* no adaptations yet */ }
  }, [session?.id]);

  // Poll every 10s
  useEffect(() => {
    fetchPlan();
    fetchAdaptations();
    const interval = setInterval(() => {
      fetchPlan();
      fetchAdaptations();
    }, 10000);
    return () => clearInterval(interval);
  }, [fetchPlan, fetchAdaptations]);

  const handleGeneratePlan = async () => {
    if (!projectPath) return;
    setLoading(true);
    try {
      const p = await api.generateCockpitPlan(projectPath, '{}');
      setPlan(p);
    } catch { /* error */ }
    setLoading(false);
  };

  const toggleCategory = (cat: string) => {
    setExpandedCategories(prev => {
      const next = new Set(prev);
      if (next.has(cat)) next.delete(cat);
      else next.add(cat);
      return next;
    });
  };

  if (!plan) {
    return (
      <div className="flex items-center justify-center h-full p-8">
        <div className="text-center space-y-3">
          <div className="text-[10px] text-gray-500 font-mono">No orchestration plan generated</div>
          <button
            onClick={handleGeneratePlan}
            disabled={loading || !projectPath}
            className="text-[9px] font-mono px-3 py-1.5 rounded bg-neon-green/10 text-neon-green hover:bg-neon-green/20 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
          >
            {loading ? 'Generating...' : 'Generate COP'}
          </button>
        </div>
      </div>
    );
  }

  const meta = plan.metaAgentConfig;
  const specialists = plan.specialistTriggers;
  const productions = plan.transverseProductions;

  return (
    <div className="p-3 space-y-3 text-[9px] font-mono">
      {/* COP Summary */}
      <div className="p-2 rounded bg-gray-800/30 border border-gray-800 space-y-1.5">
        <div className="text-[8px] font-bold text-gray-500 uppercase tracking-wider">COP Summary</div>
        <div className="flex items-center gap-2 flex-wrap">
          <span className="px-1.5 py-0.5 rounded bg-neon-purple/10 text-neon-purple text-[8px] font-bold uppercase">
            {plan.projectType}
          </span>
          <span className="px-1.5 py-0.5 rounded bg-cyan-400/10 text-cyan-400 text-[8px] font-bold uppercase">
            {plan.domain}
          </span>
        </div>
        <div className="text-gray-400">{plan.marketContext}</div>
      </div>

      {/* META Agent Status */}
      <div className="p-2 rounded bg-gray-800/30 border border-gray-800 space-y-1.5">
        <div className="flex items-center gap-2">
          <div className="text-[8px] font-bold text-gray-500 uppercase tracking-wider">META Agent</div>
          <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse shadow-emerald-400/50 shadow-sm" />
          <span className="text-emerald-400">{meta.autonomyLevel}</span>
        </div>
        <div className="flex flex-wrap gap-1">
          {meta.capabilities.map(cap => (
            <span key={cap} className="px-1.5 py-0.5 rounded bg-gray-800 text-gray-400 text-[8px]">
              {cap}
            </span>
          ))}
        </div>
        <div className="text-gray-600 text-[8px]">
          Monitoring: {Math.round(meta.monitoringIntervalMs / 1000)}s interval
        </div>
      </div>

      {/* Transverse Production / Deliverables */}
      <div className="p-2 rounded bg-gray-800/30 border border-gray-800 space-y-1.5">
        <div className="text-[8px] font-bold text-gray-500 uppercase tracking-wider">Deliverables</div>
        {productions.map(prod => {
          const isExpanded = expandedCategories.has(prod.category);
          return (
            <div key={prod.category}>
              <button
                onClick={() => toggleCategory(prod.category)}
                className="flex items-center gap-1.5 w-full text-left hover:bg-gray-800/30 rounded px-1 py-0.5 transition-colors"
              >
                <span className="text-gray-600">{isExpanded ? '\u25BC' : '\u25B6'}</span>
                <span className="text-gray-300">{prod.category}/</span>
                <span className={`px-1 py-0.5 rounded text-[7px] ${
                  prod.priority === 'high' ? 'bg-red-400/10 text-red-400' :
                  prod.priority === 'medium' ? 'bg-amber-400/10 text-amber-400' :
                  'bg-gray-800 text-gray-500'
                }`}>{prod.priority}</span>
                <span className="text-gray-600 ml-auto">{prod.deliverables.length}</span>
              </button>
              {isExpanded && (
                <div className="ml-4 space-y-0.5 mt-0.5">
                  {prod.deliverables.map(d => (
                    <div key={d} className="flex items-center gap-1.5 text-gray-400 py-0.5">
                      <span className="text-gray-600">{'\u2610'}</span>
                      <span>{d}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>

      {/* Specialist Status */}
      <div className="p-2 rounded bg-gray-800/30 border border-gray-800 space-y-1.5">
        <div className="text-[8px] font-bold text-gray-500 uppercase tracking-wider">Specialists</div>
        {specialists.length === 0 ? (
          <div className="text-gray-600">No specialists triggered</div>
        ) : (
          specialists.map((sp, i) => (
            <div key={i} className="flex items-center gap-2 py-0.5">
              <span className="w-1.5 h-1.5 rounded-full bg-cyan-400" />
              <span className="text-gray-300">{sp.specialist}</span>
              <span className="text-gray-600 truncate flex-1">{sp.condition}</span>
            </div>
          ))
        )}
      </div>

      {/* Adaptation Log */}
      <div className="p-2 rounded bg-gray-800/30 border border-gray-800 space-y-1">
        <div className="text-[8px] font-bold text-gray-500 uppercase tracking-wider">Adaptation Log</div>
        {adaptations.length === 0 ? (
          <div className="text-gray-600">No adaptations recorded</div>
        ) : (
          adaptations.slice(-10).map((a, i) => (
            <div key={i} className="flex items-start gap-2 py-0.5">
              <span className="text-neon-green shrink-0">{'\u25B8'}</span>
              <span className="text-gray-400">{a.action}</span>
              {a.targetSlot && <span className="text-gray-600">@{a.targetSlot}</span>}
              <span className="text-gray-600 truncate">{a.reason}</span>
            </div>
          ))
        )}
      </div>

      {/* Deliverables Path */}
      <div className="text-[8px] text-gray-600 px-1">
        {plan.deliverablesPath}
      </div>
    </div>
  );
}
