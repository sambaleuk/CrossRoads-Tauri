import type { AgentSlotStatus } from '../models';

interface SlotPosition {
  x: number;
  y: number;
}

interface SlotInfo {
  status: AgentSlotStatus;
  agentType: string;
}

interface Props {
  centerX: number;
  centerY: number;
  slotPositions: SlotPosition[];
  slots: SlotInfo[];
  width: number;
  height: number;
}

const agentColors: Record<string, string> = {
  claude: '#0DF170',
  gemini: '#00D4FF',
  codex: '#8B5CF6',
};

const dimColor = '#374151'; // gray-700

export function SynapseConnections({ centerX, centerY, slotPositions, slots, width, height }: Props) {
  return (
    <svg
      width={width}
      height={height}
      className="absolute inset-0 pointer-events-none"
      style={{ zIndex: 0 }}
    >
      <defs>
        <filter id="synapse-glow" x="-20%" y="-20%" width="140%" height="140%">
          <feGaussianBlur stdDeviation="2" result="blur" />
          <feMerge>
            <feMergeNode in="blur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      {slotPositions.map((pos, i) => {
        const slot = slots[i];
        if (!slot) return null;

        const isActive = slot.status === 'running' || slot.status === 'provisioning';
        const isWaiting = slot.status === 'waiting_approval';
        const isEmpty = slot.status === 'empty';
        const isError = slot.status === 'error';
        const isDone = slot.status === 'done';

        const color = isEmpty ? dimColor
          : isError ? '#EF4444'
          : isDone ? '#10B981'
          : agentColors[slot.agentType] ?? '#0DF170';

        const opacity = isEmpty ? 0.2 : isActive ? 0.8 : 0.4;

        return (
          <g key={i} filter={isActive ? 'url(#synapse-glow)' : undefined}>
            {/* Main line */}
            <line
              x1={centerX}
              y1={centerY}
              x2={pos.x}
              y2={pos.y}
              stroke={color}
              strokeWidth={isActive ? 2 : 1}
              strokeOpacity={opacity}
              strokeDasharray={isEmpty ? '4 4' : isWaiting ? '6 3' : 'none'}
            />

            {/* Animated flowing dash for active slots */}
            {isActive && (
              <line
                x1={centerX}
                y1={centerY}
                x2={pos.x}
                y2={pos.y}
                stroke={color}
                strokeWidth={2}
                strokeOpacity={0.6}
                strokeDasharray="8 12"
              >
                <animate
                  attributeName="stroke-dashoffset"
                  from="0"
                  to="-40"
                  dur="1.5s"
                  repeatCount="indefinite"
                />
              </line>
            )}

            {/* Connection endpoint dot */}
            {!isEmpty && (
              <circle
                cx={pos.x}
                cy={pos.y}
                r={3}
                fill={color}
                fillOpacity={opacity}
              >
                {isActive && (
                  <animate
                    attributeName="r"
                    values="2;4;2"
                    dur="2s"
                    repeatCount="indefinite"
                  />
                )}
              </circle>
            )}
          </g>
        );
      })}
    </svg>
  );
}
