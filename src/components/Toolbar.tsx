
import { useAppStore } from '../stores/appStore';
import { SessionCostSummary } from './CostBadge';

export function Toolbar() {
  const { showCockpit, toggleCockpit, toggleInspector, toggleChat, session, sessionCost } = useAppStore();

  return (
    <div className="flex items-center gap-2 px-4 py-2 border-b border-gray-800 bg-gray-950/80">
      {/* Left: Status */}
      <div className="flex items-center gap-2">
        <span className="w-2 h-2 rounded-full bg-emerald-400 shadow-[0_0_6px_theme(colors.emerald.400)]" />
        <span className="text-[11px] font-bold font-mono text-gray-300">READY</span>
      </div>

      <div className="flex-1" />

      {/* Center: Session cost */}
      {session && <SessionCostSummary summary={sessionCost} />}

      <div className="flex-1" />

      {/* Right: Action buttons */}
      <div className="flex items-center gap-1">
        <ToolbarButton label="Chat" icon="💬" onClick={toggleChat} shortcut="⌘⇧O" />
        <ToolbarButton
          label={showCockpit ? 'Hide Cockpit' : 'Cockpit'}
          icon={showCockpit ? '⏱' : '🎛'}
          onClick={toggleCockpit}
          active={showCockpit}
          shortcut="⌘⇧C"
        />
        <ToolbarButton label="Inspector" icon="📋" onClick={toggleInspector} shortcut="⌘⇧I" />
      </div>
    </div>
  );
}

function ToolbarButton({ label, icon, onClick, active, shortcut }: {
  label: string; icon: string; onClick: () => void; active?: boolean; shortcut?: string;
}) {
  return (
    <button
      onClick={onClick}
      title={`${label} (${shortcut})`}
      className={`flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] font-mono transition-colors
        ${active ? 'bg-neon-green/10 text-neon-green border border-neon-green/30' : 'text-gray-400 hover:text-gray-200 hover:bg-gray-800'}`}
    >
      <span>{icon}</span>
      <span>{label}</span>
    </button>
  );
}
