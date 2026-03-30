import { useState } from 'react';
import { useAppStore } from '../stores/appStore';
import * as api from '../services/api';
import type { AgentType } from '../models';

interface Story {
  id: string;
  title: string;
  priority: string;
}

interface SlotAssignment {
  slotIndex: number;
  agentType: AgentType;
  action: string;
  branch: string;
  storyIds: string[];
}

interface Props {
  featureName: string;
  stories: Story[];
  prdJson: string;
  projectPath: string;
  onClose: () => void;
  onLaunched: () => void;
}

const AGENTS: { type: AgentType; label: string; icon: string }[] = [
  { type: 'claude', label: 'Claude Code', icon: '🧠' },
  { type: 'gemini', label: 'Gemini CLI', icon: '✨' },
  { type: 'codex', label: 'Codex', icon: '⌨️' },
];

const ACTIONS = ['Implement', 'Review', 'Integration Test', 'Write Docs', 'Custom', 'Debug'];

const PRIORITY_COLORS: Record<string, string> = {
  high: 'text-red-400 bg-red-400/10',
  medium: 'text-amber-400 bg-amber-400/10',
  low: 'text-emerald-400 bg-emerald-400/10',
};

/**
 * Slot Assignment Dialog — full-screen story-to-slot assignment.
 * Matches Swift SlotAssignmentSheet behavior.
 */
export function SlotAssignmentDialog({ featureName, stories, prdJson: _prdJson, projectPath, onClose, onLaunched }: Props) {
  const [slots, setSlots] = useState<SlotAssignment[]>(() => autoAssign(stories));
  const [selectedSlot, setSelectedSlot] = useState(0);
  const [isLaunching, setIsLaunching] = useState(false);
  const [error, setError] = useState('');

  const unassigned = stories.filter(s => !slots.some(sl => sl.storyIds.includes(s.id)));
  const totalAssigned = slots.reduce((sum, sl) => sum + sl.storyIds.length, 0);

  // Toggle story assignment to selected slot
  const toggleStory = (storyId: string) => {
    setSlots(prev => {
      const updated = prev.map(sl => ({
        ...sl,
        storyIds: sl.storyIds.filter(id => id !== storyId),
      }));
      const current = updated[selectedSlot];
      if (current && !prev[selectedSlot].storyIds.includes(storyId)) {
        current.storyIds.push(storyId);
      }
      return [...updated];
    });
  };

  // Update slot config
  const updateSlot = (index: number, field: string, value: string) => {
    setSlots(prev => prev.map((sl, i) => i === index ? { ...sl, [field]: value } : sl));
  };

  // Add a slot
  const addSlot = () => {
    if (slots.length >= 6) return;
    setSlots(prev => [...prev, {
      slotIndex: prev.length,
      agentType: 'claude' as AgentType,
      action: 'Implement',
      branch: `feat/${featureName.toLowerCase().replace(/[^a-z0-9]+/g, '-')}-slot-${prev.length}`,
      storyIds: [],
    }]);
  };

  // Launch
  const launch = async () => {
    const activeSlots = slots.filter(sl => sl.storyIds.length > 0);
    if (activeSlots.length === 0) { setError('Assign at least one story to a slot'); return; }

    setIsLaunching(true);
    setError('');

    try {
      // Create session
      const session = await api.createSession(projectPath);
      useAppStore.getState().setSession(session);

      // Activate cockpit
      await api.cockpitActivate(session.id);

      // Create slots
      const createdSlots = [];
      for (const sl of activeSlots) {
        const created = await api.createSlot(session.id, sl.slotIndex, sl.agentType, undefined, sl.branch);
        await api.updateSlot(created.id, 'provisioning', sl.storyIds.join(', '));
        createdSlots.push(created);
      }
      useAppStore.getState().setSlots(createdSlots);
      useAppStore.getState().toggleCockpit();

      onLaunched();
    } catch (err: any) {
      setError(err?.message ?? 'Failed to launch');
    } finally {
      setIsLaunching(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-gray-950 flex flex-col">
      {/* Header */}
      <div className="flex items-center gap-4 px-6 py-3 border-b border-gray-800 bg-gray-900">
        <h2 className="text-sm font-bold font-mono text-gray-100">Assign Stories to Slots</h2>
        <span className="text-[10px] font-mono text-gray-500">
          {featureName} — {totalAssigned}/{stories.length} assigned — {slots.filter(s => s.storyIds.length > 0).length} slots
        </span>
        <div className="flex-1" />
        <button onClick={onClose} className="text-gray-500 hover:text-gray-300 text-lg">×</button>
      </div>

      {/* Main content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left: Story list */}
        <div className="w-72 border-r border-gray-800 flex flex-col">
          <div className="px-4 py-2 border-b border-gray-800">
            <div className="text-[9px] font-bold font-mono text-gray-500 uppercase">Stories ({stories.length})</div>
          </div>
          <div className="flex-1 overflow-auto p-2 space-y-1">
            {stories.map(story => {
              const assignedTo = slots.findIndex(sl => sl.storyIds.includes(story.id));
              const isAssigned = assignedTo >= 0;
              return (
                <button key={story.id} onClick={() => toggleStory(story.id)}
                  className={`w-full text-left p-2 rounded text-[10px] font-mono transition-colors ${
                    isAssigned
                      ? 'bg-neon-green/10 border border-neon-green/30'
                      : 'bg-gray-800/40 border border-gray-800 hover:border-gray-700'
                  }`}>
                  <div className="flex items-center gap-2">
                    <span className={`w-3 h-3 rounded-sm flex items-center justify-center text-[8px] ${
                      isAssigned ? 'bg-neon-green text-gray-950' : 'border border-gray-600'
                    }`}>
                      {isAssigned && '✓'}
                    </span>
                    <span className={`px-1 py-0.5 rounded text-[7px] font-bold uppercase ${PRIORITY_COLORS[story.priority] ?? 'text-gray-500 bg-gray-800'}`}>
                      {story.priority}
                    </span>
                    <span className="text-gray-500">{story.id}</span>
                  </div>
                  <div className="text-gray-300 mt-1 pl-5">{story.title}</div>
                  {isAssigned && (
                    <div className="text-[8px] text-neon-green mt-0.5 pl-5">→ Slot {assignedTo + 1}</div>
                  )}
                </button>
              );
            })}
          </div>
        </div>

        {/* Center: Slot cards */}
        <div className="flex-1 p-6 overflow-auto">
          <div className="grid grid-cols-3 gap-4">
            {slots.map((slot, i) => (
              <button key={i} onClick={() => setSelectedSlot(i)}
                className={`p-4 rounded-lg border text-left transition-all ${
                  selectedSlot === i
                    ? 'border-neon-green/50 bg-neon-green/5 shadow-[0_0_12px_var(--xr-neon-green)]'
                    : 'border-gray-700 bg-gray-800/40 hover:border-gray-600'
                }`}>
                <div className="flex items-center gap-2 mb-2">
                  <span className="text-[10px] font-bold font-mono text-gray-400 bg-gray-800 px-1.5 py-0.5 rounded">
                    Slot {i + 1}
                  </span>
                  <span className="text-xs">{AGENTS.find(a => a.type === slot.agentType)?.icon}</span>
                  <span className="text-[9px] font-mono text-gray-400">{slot.agentType}</span>
                </div>
                <div className="text-[10px] font-bold font-mono text-neon-green mb-1">{slot.storyIds.length} stories</div>
                <div className="space-y-0.5">
                  {slot.storyIds.slice(0, 4).map(id => {
                    const story = stories.find(s => s.id === id);
                    return (
                      <div key={id} className="text-[8px] font-mono text-gray-500 truncate">
                        {id}: {story?.title ?? ''}
                      </div>
                    );
                  })}
                  {slot.storyIds.length > 4 && (
                    <div className="text-[8px] font-mono text-gray-600">+{slot.storyIds.length - 4} more</div>
                  )}
                </div>
              </button>
            ))}

            {/* Add slot button */}
            {slots.length < 6 && (
              <button onClick={addSlot}
                className="p-4 rounded-lg border border-dashed border-gray-700 hover:border-neon-green/40 flex items-center justify-center text-gray-600 hover:text-neon-green transition-colors">
                <span className="text-lg">+</span>
              </button>
            )}
          </div>
        </div>

        {/* Right: Slot config */}
        <div className="w-64 border-l border-gray-800 p-4 space-y-4">
          <div className="text-[9px] font-bold font-mono text-gray-500 uppercase">Slot {selectedSlot + 1} Config</div>

          {/* Agent */}
          <div className="space-y-1.5">
            <div className="text-[8px] font-mono text-gray-500 uppercase">Agent</div>
            <div className="space-y-1">
              {AGENTS.map(a => (
                <button key={a.type} onClick={() => updateSlot(selectedSlot, 'agentType', a.type)}
                  className={`w-full flex items-center gap-2 px-2 py-1.5 rounded text-[10px] font-mono transition-colors ${
                    slots[selectedSlot]?.agentType === a.type
                      ? 'bg-neon-green/15 text-neon-green border border-neon-green/30'
                      : 'bg-gray-800 text-gray-400 border border-gray-700 hover:border-gray-600'
                  }`}>
                  <span>{a.icon}</span>
                  <span>{a.label}</span>
                </button>
              ))}
            </div>
          </div>

          {/* Action */}
          <div className="space-y-1.5">
            <div className="text-[8px] font-mono text-gray-500 uppercase">Action</div>
            <select value={slots[selectedSlot]?.action ?? 'Implement'}
              onChange={e => updateSlot(selectedSlot, 'action', e.target.value)}
              className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-1.5 text-[10px] font-mono text-gray-300 focus:border-neon-green/40 focus:outline-none">
              {ACTIONS.map(a => <option key={a} value={a}>{a}</option>)}
            </select>
          </div>

          {/* Branch */}
          <div className="space-y-1.5">
            <div className="text-[8px] font-mono text-gray-500 uppercase">Branch</div>
            <input value={slots[selectedSlot]?.branch ?? ''}
              onChange={e => updateSlot(selectedSlot, 'branch', e.target.value)}
              className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-1.5 text-[10px] font-mono text-gray-300 focus:border-neon-green/40 focus:outline-none" />
          </div>

          {/* Stories in this slot */}
          <div className="space-y-1.5">
            <div className="text-[8px] font-mono text-gray-500 uppercase">
              {slots[selectedSlot]?.storyIds.length ?? 0} stories
            </div>
            <div className="space-y-1 max-h-32 overflow-auto">
              {slots[selectedSlot]?.storyIds.map(id => (
                <div key={id} className="flex items-center gap-1 text-[9px] font-mono text-gray-400">
                  <span className="text-neon-green">✓</span>
                  <span>{id}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Footer */}
      <div className="px-6 py-3 border-t border-gray-800 bg-gray-900 flex items-center justify-between">
        <div>
          {error && <span className="text-[10px] font-mono text-red-400">{error}</span>}
          {unassigned.length > 0 && !error && (
            <span className="text-[10px] font-mono text-amber-400">{unassigned.length} stories unassigned</span>
          )}
        </div>
        <div className="flex gap-3">
          <button onClick={onClose}
            className="px-4 py-2 text-[11px] font-mono text-gray-400 hover:text-gray-200 transition-colors">
            Cancel
          </button>
          <button onClick={launch} disabled={isLaunching}
            className="px-5 py-2 text-[11px] font-mono bg-neon-green/20 text-neon-green border border-neon-green/30 rounded hover:bg-neon-green/30 transition-colors disabled:opacity-50">
            {isLaunching ? 'Launching...' : 'Start Launch'}
          </button>
        </div>
      </div>
    </div>
  );
}

/**
 * Auto-assign stories to slots based on priority.
 * High priority → Slot 1 (claude), Medium → Slot 2, Low → Slot 3
 */
function autoAssign(stories: Story[]): SlotAssignment[] {
  const high = stories.filter(s => s.priority === 'high').map(s => s.id);
  const medium = stories.filter(s => s.priority === 'medium').map(s => s.id);
  const low = stories.filter(s => s.priority === 'low' || !['high', 'medium', 'low'].includes(s.priority)).map(s => s.id);

  const slots: SlotAssignment[] = [];

  if (high.length > 0) {
    slots.push({ slotIndex: 0, agentType: 'claude', action: 'Implement', branch: 'feat/slot-1-critical', storyIds: high });
  }
  if (medium.length > 0) {
    slots.push({ slotIndex: slots.length, agentType: 'claude', action: 'Implement', branch: 'feat/slot-2-features', storyIds: medium });
  }
  if (low.length > 0) {
    slots.push({ slotIndex: slots.length, agentType: 'gemini', action: 'Implement', branch: 'feat/slot-3-polish', storyIds: low });
  }

  // Ensure at least one slot
  if (slots.length === 0) {
    slots.push({ slotIndex: 0, agentType: 'claude', action: 'Implement', branch: 'feat/slot-1', storyIds: stories.map(s => s.id) });
  }

  return slots;
}
