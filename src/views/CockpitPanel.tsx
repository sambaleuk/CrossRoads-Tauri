import { useState } from 'react';
import { useAppStore } from '../stores/appStore';
import { StatusBadge } from '../components/StatusBadge';
import { CostBadge } from '../components/CostBadge';
import { SessionCostSummary } from '../components/CostBadge';
import type { AgentSlot, ExecutionGate, UsageSummary } from '../models';

// Slot card within cockpit
function CockpitSlotCard({ slot, cost, gate, onApprove, onReject }: {
  slot: AgentSlot;
  cost?: UsageSummary;
  gate?: ExecutionGate;
  onApprove?: (gate: ExecutionGate) => void;
  onReject?: (gate: ExecutionGate) => void;
}) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="rounded-lg border border-gray-800 bg-gray-900/60 overflow-hidden">
      {/* Header */}
      <button onClick={() => setExpanded(!expanded)} className="w-full flex items-center gap-2 px-3 py-2 hover:bg-gray-800/30 transition-colors">
        <span className="text-[10px] font-bold font-mono text-gray-500 bg-gray-800 px-1.5 py-0.5 rounded">#{slot.slotIndex + 1}</span>
        <span className="text-xs font-semibold font-mono text-gray-200 truncate flex-1 text-left">
          {slot.branchName?.split('/').pop() ?? `slot-${slot.slotIndex}`}
        </span>
        <StatusBadge status={slot.status} />
        <span className="text-[9px] text-gray-600">{expanded ? '▲' : '▼'}</span>
      </button>

      {/* Details */}
      <div className="px-3 pb-2 space-y-1.5">
        <div className="flex items-center gap-1.5 text-[10px] text-gray-400 font-mono">
          <span>{slot.agentType === 'claude' ? '🧠' : slot.agentType === 'gemini' ? '✨' : '⌨️'}</span>
          <span>{slot.agentType}</span>
        </div>
        {cost && <CostBadge summary={cost} />}
      </div>

      {/* Approval card */}
      {gate && gate.status === 'awaiting_approval' && (
        <div className={`mx-2 mb-2 p-2.5 rounded-lg border ${
          gate.riskLevel === 'critical' ? 'border-red-500/40 bg-red-500/5' :
          gate.riskLevel === 'high' ? 'border-amber-500/40 bg-amber-500/5' :
          'border-yellow-500/40 bg-yellow-500/5'
        }`}>
          <div className="flex items-center gap-2 mb-2">
            <span className="text-[9px] font-bold font-mono text-red-400 bg-red-400/10 px-1.5 py-0.5 rounded-full uppercase">
              {gate.riskLevel}
            </span>
            <span className="text-[10px] font-mono text-gray-300 truncate">{gate.operationType}</span>
          </div>
          <div className="text-[9px] font-mono text-gray-500 mb-2 truncate">{gate.operationPayload}</div>
          <div className="flex gap-2">
            <button onClick={() => onApprove?.(gate)} className="flex-1 text-[10px] font-mono py-1 rounded bg-emerald-500/20 text-emerald-400 hover:bg-emerald-500/30">
              Approve
            </button>
            <button onClick={() => onReject?.(gate)} className="flex-1 text-[10px] font-mono py-1 rounded bg-red-500/20 text-red-400 hover:bg-red-500/30">
              Reject
            </button>
          </div>
        </div>
      )}

      {/* Chat (expanded) */}
      {expanded && (
        <div className="border-t border-gray-800 p-2">
          <div className="text-[9px] text-gray-600 font-mono text-center py-4">
            Slot chat — messages stream here
          </div>
        </div>
      )}
    </div>
  );
}

// Chairman Feed sidebar
function ChairmanFeed({ brief }: { brief?: string }) {
  return (
    <div className="border-l border-gray-800 w-64 flex flex-col">
      <div className="p-2 border-b border-gray-800 flex items-center gap-1.5">
        <span className="text-xs">👑</span>
        <span className="text-[9px] font-bold font-mono text-gray-400">CHAIRMAN BRIEF</span>
      </div>
      <div className="flex-1 overflow-auto p-2">
        {brief ? (
          <pre className="text-[10px] font-mono text-gray-400 whitespace-pre-wrap">{brief}</pre>
        ) : (
          <p className="text-[9px] text-gray-600 font-mono text-center mt-8">No brief yet</p>
        )}
      </div>
    </div>
  );
}

// Audit trail (simple list)
function AuditTrail({ gates }: { gates: ExecutionGate[] }) {
  return (
    <div className="border-t border-gray-800 max-h-48 overflow-auto">
      <div className="p-2">
        <h3 className="text-[9px] font-bold font-mono text-gray-500 mb-1.5">AUDIT TRAIL</h3>
        {gates.length === 0 ? (
          <p className="text-[9px] text-gray-600 font-mono">No gates recorded</p>
        ) : (
          <div className="space-y-1">
            {gates.map((g) => (
              <div key={g.id} className="flex items-center gap-2 text-[9px] font-mono">
                <span className={g.status === 'completed' ? 'text-emerald-400' : g.status === 'rejected' ? 'text-red-400' : 'text-amber-400'}>●</span>
                <span className="text-gray-400 truncate flex-1">{g.operationType}</span>
                <span className={`px-1 py-0.5 rounded text-[8px] ${
                  g.riskLevel === 'critical' ? 'text-red-400 bg-red-400/10' : 'text-gray-500 bg-gray-800'
                }`}>{g.riskLevel}</span>
                <span className="text-gray-600">{g.status}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// Main cockpit panel
export function CockpitPanel() {
  const { session, slots, sessionCost, slotCosts } = useAppStore();
  const [showAudit, setShowAudit] = useState(false);
  const [gates, _setGates] = useState<ExecutionGate[]>([]);

  const status = session?.status ?? 'idle';

  const statusColor = {
    idle: 'text-gray-500', initializing: 'text-blue-400', active: 'text-emerald-400',
    paused: 'text-yellow-400', closed: 'text-gray-500',
  }[status] ?? 'text-gray-500';

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="p-3 border-b border-gray-800 flex items-center gap-2">
        <span className={`w-2 h-2 rounded-full bg-current shadow-[0_0_6px_currentColor] ${statusColor}`} />
        <span className="text-[11px] font-bold font-mono text-gray-200">COCKPIT</span>
        <span className={`text-[9px] font-mono px-1.5 py-0.5 rounded bg-current/[0.1] ${statusColor}`}>
          {status.toUpperCase()}
        </span>

        <div className="flex-1" />

        <SessionCostSummary summary={sessionCost} />

        {(status === 'active' || status === 'paused') && (
          <button onClick={() => setShowAudit(!showAudit)} className="text-[9px] font-mono text-gray-400 hover:text-gray-200 px-2 py-1 rounded hover:bg-gray-800">
            Audit Trail
          </button>
        )}
      </div>

      {/* Content: slots + chairman */}
      <div className="flex-1 flex overflow-hidden">
        {slots.length === 0 ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center space-y-2">
              <span className="text-2xl text-gray-600">🖥</span>
              <p className="text-[10px] text-gray-500 font-mono">No active cockpit session</p>
            </div>
          </div>
        ) : (
          <>
            <div className="flex-1 overflow-auto p-3 space-y-2">
              {slots.map((slot) => (
                <CockpitSlotCard
                  key={slot.id}
                  slot={slot}
                  cost={slotCosts[slot.id]}
                />
              ))}
            </div>
            <ChairmanFeed brief={session?.chairmanBrief ?? undefined} />
          </>
        )}
      </div>

      {/* Audit trail */}
      {showAudit && <AuditTrail gates={gates} />}
    </div>
  );
}
