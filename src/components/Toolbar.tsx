import { useState } from 'react';
import { useAppStore } from '../stores/appStore';
import { SessionCostSummary } from './CostBadge';
import { SettingsPanel } from '../views/SettingsPanel';
import { SkillsBrowser } from '../views/SkillsBrowser';

type OrchestratorStatus = 'ready' | 'running' | 'merging' | 'paused' | 'error';

const statusConfig: Record<OrchestratorStatus, { color: string; dot: string; label: string }> = {
  ready: { color: 'emerald', dot: 'bg-emerald-400', label: 'READY' },
  running: { color: 'neon-green', dot: 'bg-neon-green', label: 'RUNNING' },
  merging: { color: 'neon-blue', dot: 'bg-neon-blue', label: 'MERGING' },
  paused: { color: 'yellow', dot: 'bg-yellow-400', label: 'PAUSED' },
  error: { color: 'red', dot: 'bg-red-400', label: 'ERROR' },
};

export function Toolbar() {
  const { showCockpit, toggleCockpit, toggleInspector, toggleChat, session, sessionCost, slots } = useAppStore();
  const [showSettings, setShowSettings] = useState(false);
  const [showSkills, setShowSkills] = useState(false);

  // Derive status from session + slots
  const status: OrchestratorStatus = !session || session.status === 'idle' ? 'ready'
    : session.status === 'paused' ? 'paused'
    : slots.some((s) => s.status === 'error') ? 'error'
    : slots.some((s) => s.status === 'running') ? 'running'
    : 'ready';

  const cfg = statusConfig[status];
  const runningCount = slots.filter((s) => s.status === 'running').length;
  const totalSlots = slots.length;

  return (
    <>
      <div className="flex items-center gap-2 px-4 py-1.5 border-b border-gray-800 bg-gray-950/80">
        {/* Left: Status + agent count */}
        <div className="flex items-center gap-2">
          <span className={`w-2 h-2 rounded-full ${cfg.dot} shadow-[0_0_6px_currentColor]`} />
          <span className="text-[11px] font-bold font-mono text-gray-300">{cfg.label}</span>
          {totalSlots > 0 && (
            <span className="text-[9px] font-mono text-gray-500">
              {runningCount}/{totalSlots} agents
            </span>
          )}
        </div>

        {/* Progress bar for running state */}
        {status === 'running' && (
          <div className="w-24 h-1 bg-gray-800 rounded-full overflow-hidden">
            <div className="h-full bg-neon-green/60 rounded-full animate-pulse" style={{ width: '60%' }} />
          </div>
        )}

        <div className="flex-1" />

        {/* Center: Session cost */}
        {session && <SessionCostSummary summary={sessionCost} />}

        <div className="flex-1" />

        {/* Right: Action buttons */}
        <div className="flex items-center gap-1">
          <ToolbarButton label="Chat" shortcut="O" onClick={toggleChat} />
          <ToolbarButton label="Cockpit" shortcut="C" onClick={toggleCockpit} active={showCockpit} />
          <ToolbarButton label="Inspector" shortcut="I" onClick={toggleInspector} />
          <ToolbarButton label="Skills" onClick={() => setShowSkills(true)} />
          <ToolbarButton label="Settings" onClick={() => setShowSettings(true)} />
        </div>
      </div>

      {showSettings && <SettingsPanel onClose={() => setShowSettings(false)} />}
      {showSkills && <SkillsBrowser onClose={() => setShowSkills(false)} />}
    </>
  );
}

export function BottomBar() {
  const { projectPath, session, slots } = useAppStore();
  const repoName = projectPath?.split('/').pop() ?? 'No project';
  const activeCount = slots.filter((s) => s.status !== 'empty').length;

  return (
    <div className="flex items-center gap-3 px-4 py-1 border-t border-gray-800 bg-gray-950/80 text-[9px] font-mono text-gray-500">
      <div className="flex items-center gap-1.5">
        <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
        <span>{repoName}</span>
      </div>
      <span className="text-gray-700">|</span>
      <span>{session?.status ?? 'idle'}</span>
      <span className="text-gray-700">|</span>
      <span>{activeCount} slots active</span>
      <div className="flex-1" />
      <span className="text-gray-600">XRoads Tauri v0.1.0</span>
    </div>
  );
}

function ToolbarButton({ label, shortcut, onClick, active }: {
  label: string; shortcut?: string; onClick: () => void; active?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      title={shortcut ? `Ctrl+Shift+${shortcut}` : undefined}
      className={`flex items-center gap-1 px-2 py-1 rounded text-[10px] font-mono transition-colors
        ${active ? 'bg-neon-green/10 text-neon-green border border-neon-green/30' : 'text-gray-400 hover:text-gray-200 hover:bg-gray-800'}`}
    >
      <span>{label}</span>
      {shortcut && (
        <span className="text-[8px] text-gray-600 ml-0.5">^{shortcut}</span>
      )}
    </button>
  );
}
