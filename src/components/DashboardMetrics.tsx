import { useAppStore } from '../stores/appStore';

/**
 * Live metrics bar for the Dashboard.
 * Shows: layer progress, total cost, estimated time, conflict count.
 * Positioned at the bottom of the dashboard, always visible during orchestration.
 */
export function DashboardMetrics() {
  const { slots, sessionCost, session } = useAppStore();

  const isActive = session?.status === 'active' || session?.status === 'initializing';
  const hasSlots = slots.length > 0 && slots.some(s => s.status !== 'empty');

  if (!isActive && !hasSlots) return null;

  // Derive metrics from slot statuses
  const total = slots.length;
  const done = slots.filter(s => s.status === 'done').length;
  const running = slots.filter(s => s.status === 'running' || s.status === 'provisioning').length;
  const errors = slots.filter(s => s.status === 'error').length;
  const waiting = slots.filter(s => s.status === 'waiting_approval').length;
  const progress = total > 0 ? done / total : 0;

  // Cost formatting
  const costCents = sessionCost.totalCostCents;
  const costStr = costCents >= 100
    ? `$${(costCents / 100).toFixed(2)}`
    : `${costCents}¢`;

  // Progress bar color
  const barColor = errors > 0
    ? 'bg-red-500'
    : progress >= 1
      ? 'bg-neon-green'
      : waiting > 0
        ? 'bg-amber-400'
        : 'bg-neon-green/70';

  // Status text
  const statusText = errors > 0
    ? `${errors} error${errors > 1 ? 's' : ''}`
    : waiting > 0
      ? `${waiting} awaiting approval`
      : running > 0
        ? `${running} running`
        : progress >= 1
          ? 'Complete'
          : 'Idle';

  return (
    <div className="absolute bottom-0 left-0 right-0 z-20">
      {/* Progress bar */}
      <div className="h-1 bg-gray-800/80">
        <div
          className={`h-full ${barColor} transition-all duration-500 ease-out`}
          style={{ width: `${Math.max(progress * 100, running > 0 ? 2 : 0)}%` }}
        />
      </div>

      {/* Metrics row */}
      <div className="flex items-center gap-4 px-4 py-1.5 bg-gray-950/90 backdrop-blur-sm border-t border-gray-800/50">
        {/* Progress */}
        <MetricItem
          label="Progress"
          value={`${done}/${total}`}
          sub={`${Math.round(progress * 100)}%`}
          color={progress >= 1 ? 'text-neon-green' : 'text-gray-300'}
        />

        <Separator />

        {/* Status */}
        <MetricItem
          label="Status"
          value={statusText}
          color={errors > 0 ? 'text-red-400' : waiting > 0 ? 'text-amber-400' : running > 0 ? 'text-neon-green' : 'text-gray-400'}
          dot={errors > 0 ? 'bg-red-400' : running > 0 ? 'bg-neon-green animate-pulse' : waiting > 0 ? 'bg-amber-400' : 'bg-gray-600'}
        />

        <Separator />

        {/* Cost */}
        <MetricItem
          label="Cost"
          value={costStr}
          sub={sessionCost.eventCount > 0 ? `${sessionCost.eventCount} calls` : undefined}
          color={costCents > 2500 ? 'text-red-400' : costCents > 500 ? 'text-amber-400' : 'text-neon-green'}
        />

        <Separator />

        {/* Agents */}
        <MetricItem
          label="Agents"
          value={`${running} active`}
          sub={total > 0 ? `${total} slots` : undefined}
          color={running > 0 ? 'text-neon-blue' : 'text-gray-400'}
        />

        {/* Conflicts (only show if any) */}
        {errors > 0 && (
          <>
            <Separator />
            <MetricItem
              label="Issues"
              value={`${errors} error${errors > 1 ? 's' : ''}`}
              color="text-red-400"
              dot="bg-red-400 animate-pulse"
            />
          </>
        )}
      </div>
    </div>
  );
}

function MetricItem({ label, value, sub, color = 'text-gray-300', dot }: {
  label: string;
  value: string;
  sub?: string;
  color?: string;
  dot?: string;
}) {
  return (
    <div className="flex items-center gap-2">
      {dot && <span className={`w-1.5 h-1.5 rounded-full ${dot}`} />}
      <div>
        <div className="text-[8px] font-mono text-gray-500 uppercase tracking-wider">{label}</div>
        <div className={`text-[11px] font-mono font-bold ${color} leading-tight`}>
          {value}
          {sub && <span className="text-[9px] font-normal text-gray-500 ml-1">{sub}</span>}
        </div>
      </div>
    </div>
  );
}

function Separator() {
  return <div className="w-px h-6 bg-gray-800" />;
}
