import { useState, useEffect } from 'react';
import { useAppStore } from '../stores/appStore';
import { StatusBadge } from '../components/StatusBadge';
import { CostBadge, SessionCostSummary } from '../components/CostBadge';
import { CockpitBrainTab } from '../components/CockpitBrainTab';
import * as api from '../services/api';
import type { AgentSlot, ExecutionGate, UsageSummary, OrgRole, BudgetStatus, TrustScore } from '../models';

type CockpitTab = 'brain' | 'slots' | 'org' | 'budget' | 'health' | 'audit' | 'trust';

export function CockpitPanel() {
  const { session, slots, sessionCost, slotCosts } = useAppStore();
  const [tab, setTab] = useState<CockpitTab>('brain');
  const [gates, setGates] = useState<ExecutionGate[]>([]);
  const [orgRoles, setOrgRoles] = useState<OrgRole[]>([]);
  const [budgetStatus, setBudgetStatus] = useState<BudgetStatus | null>(null);
  const [trustScores, setTrustScores] = useState<TrustScore[]>([]);

  const status = session?.status ?? 'idle';
  const isActive = status === 'active' || status === 'paused';

  // Poll for gates, org, budget, trust
  useEffect(() => {
    if (!session?.id || !isActive) return;
    const interval = setInterval(async () => {
      // Fetch pending gates from all slots
      const allGates: ExecutionGate[] = [];
      for (const slot of slots) {
        try {
          const sg = await api.fetchGates(slot.id);
          allGates.push(...sg);
        } catch { /* skip */ }
      }
      setGates(allGates);

      // Org roles
      try { setOrgRoles(await api.fetchOrgRoles(session.id)); } catch {}
      // Budget
      if (slots[0]) {
        try { setBudgetStatus(await api.checkBudget(slots[0].id)); } catch {}
      }
      // Trust
      try { setTrustScores(await api.fetchTrustScores()); } catch {}
    }, 5000);
    return () => clearInterval(interval);
  }, [session?.id, isActive, slots]);

  // Attention items: gates awaiting approval + errors + budget exceeded
  const pendingGates = gates.filter(g => g.status === 'awaiting_approval');
  const errorSlots = slots.filter(s => s.status === 'error');
  const budgetWarning = budgetStatus && (budgetStatus.status === 'warning' || budgetStatus.status === 'exceeded');
  const attentionCount = pendingGates.length + errorSlots.length + (budgetWarning ? 1 : 0);

  const handleApprove = async (gate: ExecutionGate) => {
    try { await api.safeApproveGate(gate.id, gate.agentSlotId, 'user'); } catch {}
  };
  const handleReject = async (gate: ExecutionGate) => {
    try { await api.safeRejectGate(gate.id, gate.agentSlotId, 'User rejected'); } catch {}
  };
  const handlePause = async () => { if (session) try { await api.cockpitPause(session.id); } catch {} };
  const handleResume = async () => { if (session) try { await api.cockpitResume(session.id); } catch {} };
  const handleClose = async () => { if (session) try { await api.cockpitClose(session.id); } catch {} };

  const statusColor: Record<string, string> = {
    idle: 'text-gray-500', initializing: 'text-blue-400', active: 'text-emerald-400',
    paused: 'text-yellow-400', closed: 'text-gray-500',
  };

  const tabs: { key: CockpitTab; label: string }[] = [
    { key: 'brain', label: 'Brain' },
    { key: 'slots', label: 'Slots' },
    { key: 'org', label: 'Org' },
    { key: 'budget', label: 'Budget' },
    { key: 'health', label: 'Health' },
    { key: 'trust', label: 'Trust' },
    { key: 'audit', label: 'Audit' },
  ];

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="px-3 py-2 border-b border-gray-800">
        <div className="flex items-center gap-2">
          <span className={`w-2 h-2 rounded-full bg-current shadow-[0_0_6px_currentColor] ${statusColor[status] ?? 'text-gray-500'}`} />
          <span className="text-[11px] font-bold font-mono text-gray-200">COCKPIT</span>
          <span className={`text-[9px] font-mono px-1.5 py-0.5 rounded bg-current/10 ${statusColor[status] ?? 'text-gray-500'}`}>
            {status.toUpperCase()}
          </span>
          <div className="flex-1" />
          <SessionCostSummary summary={sessionCost} />
          {isActive && (
            <div className="flex gap-1">
              {status === 'active' ? (
                <button onClick={handlePause} className="text-[9px] font-mono text-yellow-400 hover:bg-yellow-400/10 px-2 py-0.5 rounded">Pause</button>
              ) : (
                <button onClick={handleResume} className="text-[9px] font-mono text-emerald-400 hover:bg-emerald-400/10 px-2 py-0.5 rounded">Resume</button>
              )}
              <button onClick={handleClose} className="text-[9px] font-mono text-red-400 hover:bg-red-400/10 px-2 py-0.5 rounded">Close</button>
            </div>
          )}
        </div>
      </div>

      {/* Attention needed */}
      {attentionCount > 0 && (
        <div className="px-3 py-2 bg-amber-500/5 border-b border-amber-500/20">
          <div className="text-[9px] font-bold font-mono text-amber-400 mb-1.5">ATTENTION NEEDED ({attentionCount})</div>
          {pendingGates.map(gate => (
            <div key={gate.id} className={`mb-1.5 p-2 rounded border ${
              gate.riskLevel === 'critical' ? 'border-red-500/40 bg-red-500/5' :
              gate.riskLevel === 'high' ? 'border-amber-500/40 bg-amber-500/5' :
              'border-yellow-500/40 bg-yellow-500/5'
            }`}>
              <div className="flex items-center gap-2 mb-1">
                <span className="text-[8px] font-bold font-mono text-red-400 bg-red-400/10 px-1 py-0.5 rounded-full uppercase">{gate.riskLevel}</span>
                <span className="text-[10px] font-mono text-gray-300 truncate">{gate.operationType}</span>
              </div>
              <div className="text-[9px] font-mono text-gray-500 mb-1.5 truncate">{gate.operationPayload}</div>
              <div className="flex gap-2">
                <button onClick={() => handleApprove(gate)} className="flex-1 text-[9px] font-mono py-1 rounded bg-emerald-500/20 text-emerald-400 hover:bg-emerald-500/30">Approve</button>
                <button onClick={() => handleReject(gate)} className="flex-1 text-[9px] font-mono py-1 rounded bg-red-500/20 text-red-400 hover:bg-red-500/30">Reject</button>
              </div>
            </div>
          ))}
          {errorSlots.map(slot => (
            <div key={slot.id} className="mb-1.5 p-2 rounded border border-red-500/30 bg-red-500/5 flex items-center gap-2">
              <span className="text-[9px] font-mono text-red-400">Slot #{slot.slotIndex + 1} ERROR</span>
              <span className="text-[9px] font-mono text-gray-500 truncate flex-1">{slot.currentTask ?? ''}</span>
            </div>
          ))}
          {budgetWarning && budgetStatus && (
            <div className="p-2 rounded border border-amber-500/30 bg-amber-500/5 flex items-center gap-2">
              <span className="text-[9px] font-mono text-amber-400">Budget {budgetStatus.status}: {Math.round(budgetStatus.percentUsed)}%</span>
            </div>
          )}
        </div>
      )}

      {/* Tabs */}
      <div className="flex border-b border-gray-800">
        {tabs.map(t => (
          <button key={t.key} onClick={() => setTab(t.key)}
            className={`flex-1 px-2 py-1.5 text-[9px] font-mono uppercase tracking-wider transition-colors ${
              tab === t.key ? 'text-neon-green border-b border-neon-green' : 'text-gray-500 hover:text-gray-400'
            }`}>
            {t.label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div className="flex-1 overflow-auto">
        {tab === 'brain' && <CockpitBrainTab />}
        {tab === 'slots' && <SlotsTab slots={slots} slotCosts={slotCosts} session={session} />}
        {tab === 'org' && <OrgTab roles={orgRoles} />}
        {tab === 'budget' && <BudgetTab status={budgetStatus} sessionCost={sessionCost} />}
        {tab === 'health' && <HealthTab slots={slots} />}
        {tab === 'trust' && <TrustTab scores={trustScores} />}
        {tab === 'audit' && <AuditTab gates={gates} />}
      </div>
    </div>
  );
}

// ── Tab: Slots ──

function SlotsTab({ slots, slotCosts, session }: { slots: AgentSlot[]; slotCosts: Record<string, UsageSummary>; session: any }) {
  if (slots.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center p-8">
        <div className="text-center space-y-2">
          <span className="text-2xl text-gray-600">🖥</span>
          <p className="text-[10px] text-gray-500 font-mono">No active session</p>
        </div>
      </div>
    );
  }

  return (
    <div className="p-3 space-y-2">
      {session?.chairmanBrief && (
        <div className="p-2 rounded bg-gray-800/30 border border-gray-800 mb-3">
          <div className="text-[8px] font-mono text-gray-500 uppercase mb-1">Chairman Brief</div>
          <div className="text-[10px] font-mono text-gray-400 whitespace-pre-wrap">{session.chairmanBrief}</div>
        </div>
      )}
      {slots.map(slot => (
        <div key={slot.id} className="rounded-lg border border-gray-800 bg-gray-900/60 p-2.5 space-y-1.5">
          <div className="flex items-center gap-2">
            <span className="text-[10px] font-bold font-mono text-gray-500 bg-gray-800 px-1.5 py-0.5 rounded">#{slot.slotIndex + 1}</span>
            <span className="text-[10px] font-mono text-gray-300">{slot.agentType === 'claude' ? '🧠' : slot.agentType === 'gemini' ? '✨' : '⌨️'} {slot.agentType}</span>
            <div className="flex-1" />
            <StatusBadge status={slot.status} />
          </div>
          {slot.branchName && <div className="text-[9px] font-mono text-gray-500">⎇ {slot.branchName}</div>}
          {slot.currentTask && <div className="text-[9px] font-mono text-gray-400 truncate">{slot.currentTask}</div>}
          {slotCosts[slot.id] && <CostBadge summary={slotCosts[slot.id]} />}
        </div>
      ))}
    </div>
  );
}

// ── Tab: Org Chart ──

function OrgTab({ roles }: { roles: OrgRole[] }) {
  if (roles.length === 0) {
    return <EmptyTab icon="👥" text="No org chart configured" />;
  }
  const roots = roles.filter(r => !r.parentRoleId);
  return (
    <div className="p-3 space-y-1">
      {roots.map(role => (
        <OrgNode key={role.id} role={role} roles={roles} depth={0} />
      ))}
    </div>
  );
}

function OrgNode({ role, roles, depth }: { role: OrgRole; roles: OrgRole[]; depth: number }) {
  const children = roles.filter(r => r.parentRoleId === role.id);
  return (
    <div>
      <div className="flex items-center gap-1.5 py-1" style={{ paddingLeft: depth * 16 }}>
        <span className="w-1.5 h-1.5 rounded-full bg-neon-green" />
        <span className="text-[10px] font-bold font-mono text-gray-300">{role.name}</span>
        <span className="text-[8px] font-mono text-gray-600">{role.roleType}</span>
        {role.authority === 'full' && <span className="text-[7px] font-mono text-neon-purple bg-neon-purple/10 px-1 rounded">full</span>}
      </div>
      {children.map(child => (
        <OrgNode key={child.id} role={child} roles={roles} depth={depth + 1} />
      ))}
    </div>
  );
}

// ── Tab: Budget ──

function BudgetTab({ status, sessionCost }: { status: BudgetStatus | null; sessionCost: UsageSummary }) {
  const costStr = sessionCost.totalCostCents >= 100
    ? `$${(sessionCost.totalCostCents / 100).toFixed(2)}`
    : `${sessionCost.totalCostCents}¢`;

  return (
    <div className="p-3 space-y-3">
      <div className="space-y-1">
        <div className="text-[8px] font-mono text-gray-500 uppercase">Session Cost</div>
        <div className="text-lg font-bold font-mono text-neon-green">{costStr}</div>
        <div className="text-[9px] font-mono text-gray-500">{sessionCost.eventCount} API calls</div>
      </div>
      {status && (
        <>
          <div className="space-y-1">
            <div className="flex justify-between text-[9px] font-mono">
              <span className="text-gray-500">Budget used</span>
              <span className={status.status === 'exceeded' ? 'text-red-400' : status.status === 'warning' ? 'text-amber-400' : 'text-gray-300'}>
                {Math.round(status.percentUsed)}%
              </span>
            </div>
            <div className="h-2 bg-gray-800 rounded-full overflow-hidden">
              <div className={`h-full rounded-full transition-all ${
                status.status === 'exceeded' ? 'bg-red-500' : status.status === 'warning' ? 'bg-amber-400' : 'bg-neon-green'
              }`} style={{ width: `${Math.min(status.percentUsed, 100)}%` }} />
            </div>
          </div>
          <div className="text-[9px] font-mono text-gray-500">
            Remaining: {status.remainingCents >= 100 ? `$${(status.remainingCents/100).toFixed(2)}` : `${status.remainingCents}¢`}
          </div>
        </>
      )}
    </div>
  );
}

// ── Tab: Health ──

function HealthTab({ slots }: { slots: AgentSlot[] }) {
  if (slots.length === 0) return <EmptyTab icon="💓" text="No agents to monitor" />;
  return (
    <div className="p-3 space-y-2">
      {slots.map(slot => {
        const isAlive = slot.status === 'running' || slot.status === 'provisioning';
        const isError = slot.status === 'error';
        const isDone = slot.status === 'done';
        return (
          <div key={slot.id} className="flex items-center gap-2 p-2 rounded bg-gray-800/30">
            <span className={`w-2.5 h-2.5 rounded-full ${
              isAlive ? 'bg-emerald-400 animate-pulse shadow-emerald-400/50 shadow-sm' :
              isError ? 'bg-red-400' :
              isDone ? 'bg-neon-green' : 'bg-gray-600'
            }`} />
            <span className="text-[10px] font-mono text-gray-300">Slot #{slot.slotIndex + 1}</span>
            <span className="text-[9px] font-mono text-gray-500">{slot.agentType}</span>
            <div className="flex-1" />
            <span className={`text-[9px] font-mono ${isAlive ? 'text-emerald-400' : isError ? 'text-red-400' : isDone ? 'text-neon-green' : 'text-gray-500'}`}>
              {slot.status}
            </span>
          </div>
        );
      })}
    </div>
  );
}

// ── Tab: Trust ──

function TrustTab({ scores }: { scores: TrustScore[] }) {
  if (scores.length === 0) return <EmptyTab icon="🛡" text="No trust data yet — run orchestrations to build trust scores" />;
  return (
    <div className="p-3 space-y-2">
      {scores.map(ts => (
        <div key={ts.id} className="p-2 rounded bg-gray-800/30 space-y-1">
          <div className="flex items-center gap-2">
            <span className="text-[10px] font-bold font-mono text-gray-300">{ts.agentType}</span>
            <span className="text-[9px] font-mono text-gray-500">{ts.domain}</span>
            <div className="flex-1" />
            <span className={`text-[11px] font-bold font-mono ${ts.score >= 0.8 ? 'text-neon-green' : ts.score >= 0.5 ? 'text-amber-400' : 'text-red-400'}`}>
              {Math.round(ts.score * 100)}%
            </span>
          </div>
          <div className="h-1.5 bg-gray-800 rounded-full overflow-hidden">
            <div className={`h-full rounded-full ${ts.score >= 0.8 ? 'bg-neon-green' : ts.score >= 0.5 ? 'bg-amber-400' : 'bg-red-400'}`}
              style={{ width: `${ts.score * 100}%` }} />
          </div>
          <div className="flex gap-3 text-[8px] font-mono text-gray-500">
            <span>{ts.totalStories} stories</span>
            <span>{ts.successfulStories} success</span>
            {ts.autoMergeEnabled && <span className="text-neon-green">auto-merge ✓</span>}
          </div>
        </div>
      ))}
    </div>
  );
}

// ── Tab: Audit ──

function AuditTab({ gates }: { gates: ExecutionGate[] }) {
  if (gates.length === 0) return <EmptyTab icon="📋" text="No gates recorded" />;
  return (
    <div className="p-3 space-y-1">
      {gates.map(g => (
        <div key={g.id} className="flex items-center gap-2 text-[9px] font-mono py-1">
          <span className={g.status === 'completed' ? 'text-emerald-400' : g.status === 'rejected' ? 'text-red-400' : 'text-amber-400'}>●</span>
          <span className="text-gray-400 truncate flex-1">{g.operationType}</span>
          <span className={`px-1 py-0.5 rounded text-[8px] ${
            g.riskLevel === 'critical' ? 'text-red-400 bg-red-400/10' : g.riskLevel === 'high' ? 'text-amber-400 bg-amber-400/10' : 'text-gray-500 bg-gray-800'
          }`}>{g.riskLevel}</span>
          <span className="text-gray-600">{g.status}</span>
        </div>
      ))}
    </div>
  );
}

// ── Empty state ──

function EmptyTab({ icon, text }: { icon: string; text: string }) {
  return (
    <div className="flex items-center justify-center h-full p-8">
      <div className="text-center space-y-2">
        <span className="text-xl">{icon}</span>
        <p className="text-[10px] text-gray-500 font-mono">{text}</p>
      </div>
    </div>
  );
}
