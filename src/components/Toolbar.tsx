import { useState } from 'react';
import { useAppStore } from '../stores/appStore';
import { SessionCostSummary } from './CostBadge';
import { SettingsPanel } from '../views/SettingsPanel';
import { SkillsBrowser } from '../views/SkillsBrowser';

export function Toolbar() {
  const { showCockpit, toggleCockpit, toggleInspector, toggleChat, session, sessionCost } = useAppStore();
  const [showSettings, setShowSettings] = useState(false);
  const [showSkills, setShowSkills] = useState(false);

  return (
    <>
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
          <ToolbarButton label="Chat" icon="💬" onClick={toggleChat} />
          <ToolbarButton
            label={showCockpit ? 'Hide Cockpit' : 'Cockpit'}
            icon="🎛"
            onClick={toggleCockpit}
            active={showCockpit}
          />
          <ToolbarButton label="Inspector" icon="📋" onClick={toggleInspector} />
          <ToolbarButton label="Skills" icon="🧩" onClick={() => setShowSkills(true)} />
          <ToolbarButton label="Settings" icon="⚙" onClick={() => setShowSettings(true)} />
        </div>
      </div>

      {showSettings && <SettingsPanel onClose={() => setShowSettings(false)} />}
      {showSkills && <SkillsBrowser onClose={() => setShowSkills(false)} />}
    </>
  );
}

function ToolbarButton({ label, icon, onClick, active }: {
  label: string; icon: string; onClick: () => void; active?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      className={`flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] font-mono transition-colors
        ${active ? 'bg-neon-green/10 text-neon-green border border-neon-green/30' : 'text-gray-400 hover:text-gray-200 hover:bg-gray-800'}`}
    >
      <span>{icon}</span>
      <span>{label}</span>
    </button>
  );
}
