
import type { AgentSlotStatus } from '../models';

const statusConfig: Record<string, { color: string; bg: string; label: string }> = {
  empty: { color: 'text-gray-500', bg: 'bg-gray-500/10', label: 'EMPTY' },
  provisioning: { color: 'text-blue-400', bg: 'bg-blue-400/10', label: 'PROVISIONING' },
  running: { color: 'text-emerald-400', bg: 'bg-emerald-400/10', label: 'RUNNING' },
  waiting_approval: { color: 'text-amber-400', bg: 'bg-amber-400/10', label: 'APPROVAL' },
  paused: { color: 'text-yellow-400', bg: 'bg-yellow-400/10', label: 'PAUSED' },
  done: { color: 'text-neon-green', bg: 'bg-neon-green/10', label: 'DONE' },
  error: { color: 'text-red-400', bg: 'bg-red-400/10', label: 'ERROR' },
};

export function StatusBadge({ status }: { status: AgentSlotStatus }) {
  const config = statusConfig[status] ?? statusConfig.empty;
  return (
    <span className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[9px] font-bold font-mono ${config.color} ${config.bg}`}>
      <span className={`w-1.5 h-1.5 rounded-full bg-current shadow-[0_0_6px_currentColor]`} />
      {config.label}
    </span>
  );
}
