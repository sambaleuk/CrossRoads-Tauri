
import type { UsageSummary } from '../models';

function formatCost(cents: number): string {
  return cents >= 100 ? `$${(cents / 100).toFixed(2)}` : `${cents}¢`;
}

function formatTokens(count: number): string {
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  if (count >= 1_000) return `${(count / 1_000).toFixed(1)}K`;
  return `${count}`;
}

export function CostBadge({ summary }: { summary: UsageSummary }) {
  if (summary.eventCount === 0) return null;

  const color = summary.totalCostCents >= 500
    ? 'text-red-400'
    : summary.totalCostCents >= 100
      ? 'text-amber-400'
      : 'text-emerald-400';

  return (
    <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[9px] font-bold font-mono ${color} bg-current/[0.08]`}>
      <span>$</span>
      <span>{formatCost(summary.totalCostCents)}</span>
      <span className="text-gray-500">·</span>
      <span className="text-gray-400 font-medium">{formatTokens(summary.totalInputTokens + summary.totalOutputTokens)} tok</span>
    </span>
  );
}

export function SessionCostSummary({ summary }: { summary: UsageSummary }) {
  if (summary.eventCount === 0) return null;
  return (
    <div className="flex items-center gap-3 px-3 py-1.5 bg-gray-800/50 rounded-lg text-[10px] font-mono">
      <span className="text-neon-green font-bold">{formatCost(summary.totalCostCents)}</span>
      <span className="w-px h-3 bg-gray-700" />
      <span className="text-gray-400">{formatTokens(summary.totalInputTokens)} in</span>
      <span className="text-gray-400">{formatTokens(summary.totalOutputTokens)} out</span>
    </div>
  );
}
