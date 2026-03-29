import { useState } from 'react';
import type { AgentType } from '../models';

interface Props {
  slotIndex: number;
  branches: string[];
  onConfirm: (config: SlotConfig) => void;
  onClose: () => void;
}

export interface SlotConfig {
  agentType: AgentType;
  action: string;
  branchName: string;
  skillName: string;
}

const AGENT_TYPES: { type: AgentType; label: string; icon: string; color: string }[] = [
  { type: 'claude', label: 'Claude Code', icon: '🧠', color: 'neon-green' },
  { type: 'gemini', label: 'Gemini', icon: '✨', color: 'neon-blue' },
  { type: 'codex', label: 'Codex', icon: '⌨️', color: 'neon-purple' },
];

const ACTIONS = ['Implement', 'Review', 'Test', 'Docs', 'Debug', 'Custom'];

const SKILL_FAMILIES: Record<string, string[]> = {
  Engineering: ['backend', 'frontend', 'testing', 'devops'],
  Business: ['finance-analyst', 'legal-clerk', 'hr-wiki-manager'],
  Design: ['ui-designer', 'art-director'],
};

export function SlotConfigDialog({ slotIndex, branches, onConfirm, onClose }: Props) {
  const [agentType, setAgentType] = useState<AgentType>('claude');
  const [action, setAction] = useState('Implement');
  const [branchName, setBranchName] = useState(`agent/slot-${slotIndex}`);
  const [newBranch, setNewBranch] = useState('');
  const [skillName, setSkillName] = useState('backend');

  const handleConfirm = () => {
    onConfirm({
      agentType,
      action,
      branchName: newBranch || branchName,
      skillName,
    });
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60" onClick={onClose}>
      <div
        className="w-[420px] bg-gray-900 border border-gray-700 rounded-xl shadow-2xl p-5 space-y-5"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-bold font-mono text-gray-100">Configure Slot #{slotIndex + 1}</h2>
          <button onClick={onClose} className="text-gray-500 hover:text-gray-300 text-lg">×</button>
        </div>

        {/* Agent Type */}
        <div className="space-y-2">
          <label className="text-[10px] font-mono text-gray-400 uppercase tracking-wider">Agent</label>
          <div className="flex gap-2">
            {AGENT_TYPES.map((a) => (
              <button
                key={a.type}
                onClick={() => setAgentType(a.type)}
                className={`flex-1 flex items-center gap-2 px-3 py-2 rounded-lg border text-xs font-mono transition-colors ${
                  agentType === a.type
                    ? `border-${a.color}/60 bg-${a.color}/10 text-${a.color}`
                    : 'border-gray-700 bg-gray-800/50 text-gray-400 hover:border-gray-600'
                }`}
              >
                <span>{a.icon}</span>
                <span>{a.label}</span>
              </button>
            ))}
          </div>
        </div>

        {/* Action */}
        <div className="space-y-2">
          <label className="text-[10px] font-mono text-gray-400 uppercase tracking-wider">Action</label>
          <div className="flex flex-wrap gap-1.5">
            {ACTIONS.map((a) => (
              <button
                key={a}
                onClick={() => setAction(a)}
                className={`px-3 py-1 rounded text-[11px] font-mono transition-colors ${
                  action === a
                    ? 'bg-neon-green/20 text-neon-green border border-neon-green/30'
                    : 'bg-gray-800 text-gray-400 border border-gray-700 hover:border-gray-600'
                }`}
              >
                {a}
              </button>
            ))}
          </div>
        </div>

        {/* Branch */}
        <div className="space-y-2">
          <label className="text-[10px] font-mono text-gray-400 uppercase tracking-wider">Branch</label>
          <select
            value={branchName}
            onChange={(e) => setBranchName(e.target.value)}
            className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-1.5 text-xs font-mono text-gray-200 focus:border-neon-green/40 focus:outline-none"
          >
            {branches.map((b) => (
              <option key={b} value={b}>{b}</option>
            ))}
            <option value="">+ New branch</option>
          </select>
          {branchName === '' && (
            <input
              type="text"
              placeholder="agent/slot-0-feature"
              value={newBranch}
              onChange={(e) => setNewBranch(e.target.value)}
              className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-1.5 text-xs font-mono text-gray-200 focus:border-neon-green/40 focus:outline-none mt-1"
            />
          )}
        </div>

        {/* Skill */}
        <div className="space-y-2">
          <label className="text-[10px] font-mono text-gray-400 uppercase tracking-wider">Skill</label>
          <div className="space-y-2 max-h-32 overflow-auto">
            {Object.entries(SKILL_FAMILIES).map(([family, skills]) => (
              <div key={family}>
                <div className="text-[9px] font-mono text-gray-500 uppercase mb-1">{family}</div>
                <div className="flex flex-wrap gap-1">
                  {skills.map((s) => (
                    <button
                      key={s}
                      onClick={() => setSkillName(s)}
                      className={`px-2 py-0.5 rounded text-[10px] font-mono transition-colors ${
                        skillName === s
                          ? 'bg-neon-purple/20 text-neon-purple border border-neon-purple/30'
                          : 'bg-gray-800 text-gray-400 border border-gray-700 hover:border-gray-600'
                      }`}
                    >
                      {s}
                    </button>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Actions */}
        <div className="flex justify-end gap-3 pt-2 border-t border-gray-800">
          <button
            onClick={onClose}
            className="px-4 py-1.5 text-xs font-mono text-gray-400 hover:text-gray-200 transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={handleConfirm}
            className="px-4 py-1.5 text-xs font-mono bg-neon-green/20 text-neon-green border border-neon-green/30 rounded hover:bg-neon-green/30 transition-colors"
          >
            Configure
          </button>
        </div>
      </div>
    </div>
  );
}
