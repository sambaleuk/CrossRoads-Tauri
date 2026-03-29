import type { AgentSlot, UsageSummary } from '../models';
import { StatusBadge } from './StatusBadge';
import { CostBadge } from './CostBadge';
import { SlotTerminal } from './SlotTerminal';
import { useAppStore } from '../stores/appStore';

const agentIcons: Record<string, string> = {
  claude: '🧠',
  gemini: '✨',
  codex: '⌨️',
};

interface Props {
  slot: AgentSlot;
  cost?: UsageSummary;
  isConfigured: boolean;
  onConfigure?: () => void;
}

export function TerminalSlot({ slot, cost, isConfigured, onConfigure }: Props) {
  const toggleExpandedSlot = useAppStore((s) => s.toggleExpandedSlot);
  if (!isConfigured) {
    return (
      <button
        onClick={onConfigure}
        className="flex flex-col items-center justify-center gap-3 p-6 rounded-xl border border-dashed border-gray-700 hover:border-neon-green/40 transition-colors bg-gray-900/30 min-h-[200px]"
      >
        <span className="text-2xl text-gray-600">⊕</span>
        <span className="text-xs text-gray-500 font-mono">SLOT {slot.slotIndex + 1}</span>
        <span className="text-[10px] text-gray-600 font-mono">Click to configure</span>
      </button>
    );
  }

  return (
    <div
      className="flex flex-col rounded-xl border border-gray-800 bg-gray-900/60 overflow-hidden min-h-[200px]"
      onDoubleClick={() => toggleExpandedSlot(slot.id)}
    >
      {/* Header */}
      <div className="flex items-center gap-2 px-3 py-2 border-b border-gray-800">
        <span className="text-[10px] font-bold font-mono text-gray-500 bg-gray-800 px-1.5 py-0.5 rounded">
          #{slot.slotIndex + 1}
        </span>
        <span className="text-xs font-semibold font-mono text-gray-200 truncate flex-1">
          {slot.branchName?.split('/').pop() ?? `slot-${slot.slotIndex}`}
        </span>
        <StatusBadge status={slot.status} />
      </div>

      {/* Agent info */}
      <div className="px-3 py-2 space-y-1.5">
        <div className="flex items-center gap-1.5 text-[11px]">
          <span>{agentIcons[slot.agentType] ?? '🤖'}</span>
          <span className="text-gray-400 font-mono">{slot.agentType}</span>
        </div>
        {slot.branchName && (
          <div className="flex items-center gap-1.5 text-[10px] text-gray-500 font-mono">
            <span>⎇</span>
            <span className="truncate">{slot.branchName}</span>
          </div>
        )}
        {cost && <CostBadge summary={cost} />}
        {slot.currentTask && (
          <div className="text-[10px] text-gray-400 font-mono truncate">
            {slot.currentTask}
          </div>
        )}
      </div>

      {/* Terminal output area — xterm.js */}
      <div className="flex-1 min-h-[80px]">
        <SlotTerminal slotId={slot.id} />
      </div>
    </div>
  );
}
