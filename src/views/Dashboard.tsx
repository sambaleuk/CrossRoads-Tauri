
import { useAppStore } from '../stores/appStore';
import { TerminalSlot } from '../components/TerminalSlot';
import type { AgentSlot } from '../models';

// Default 6 empty slots
const defaultSlots: AgentSlot[] = Array.from({ length: 6 }, (_, i) => ({
  id: `default-${i}`,
  cockpitSessionId: '',
  slotIndex: i,
  status: 'empty' as const,
  agentType: 'claude',
  createdAt: '',
  updatedAt: '',
}));

export function Dashboard() {
  const { slots: activeSlots, slotCosts } = useAppStore();
  const displaySlots = activeSlots.length > 0 ? activeSlots : defaultSlots;

  return (
    <div className="flex-1 overflow-auto p-6">
      {/* 6-slot grid: 1-2-3-2-1 radial layout approximated as 3x2 grid */}
      <div className="grid grid-cols-3 gap-4 max-w-5xl mx-auto">
        {/* Top row: slot 1 centered */}
        <div className="col-start-2">
          <TerminalSlot
            slot={displaySlots[0]}
            cost={slotCosts[displaySlots[0]?.id]}
            isConfigured={displaySlots[0]?.status !== 'empty'}
          />
        </div>

        {/* Middle row: slots 6, brain placeholder, slot 2 */}
        <div>
          <TerminalSlot
            slot={displaySlots[5] ?? defaultSlots[5]}
            cost={slotCosts[displaySlots[5]?.id]}
            isConfigured={(displaySlots[5]?.status ?? 'empty') !== 'empty'}
          />
        </div>
        <div className="flex items-center justify-center">
          <div className="w-24 h-24 rounded-full border-2 border-neon-green/20 flex items-center justify-center bg-neon-green/5 shadow-[0_0_30px_theme(colors.neon.green/0.1)]">
            <span className="text-3xl">🧠</span>
          </div>
        </div>
        <div>
          <TerminalSlot
            slot={displaySlots[1] ?? defaultSlots[1]}
            cost={slotCosts[displaySlots[1]?.id]}
            isConfigured={(displaySlots[1]?.status ?? 'empty') !== 'empty'}
          />
        </div>

        {/* Bottom-middle row: slots 5 and 3 */}
        <div>
          <TerminalSlot
            slot={displaySlots[4] ?? defaultSlots[4]}
            cost={slotCosts[displaySlots[4]?.id]}
            isConfigured={(displaySlots[4]?.status ?? 'empty') !== 'empty'}
          />
        </div>
        <div />
        <div>
          <TerminalSlot
            slot={displaySlots[2] ?? defaultSlots[2]}
            cost={slotCosts[displaySlots[2]?.id]}
            isConfigured={(displaySlots[2]?.status ?? 'empty') !== 'empty'}
          />
        </div>

        {/* Bottom row: slot 4 centered */}
        <div className="col-start-2">
          <TerminalSlot
            slot={displaySlots[3] ?? defaultSlots[3]}
            cost={slotCosts[displaySlots[3]?.id]}
            isConfigured={(displaySlots[3]?.status ?? 'empty') !== 'empty'}
          />
        </div>
      </div>
    </div>
  );
}
