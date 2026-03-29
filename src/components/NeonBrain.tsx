import { useMemo } from 'react';

type BrainState = 'idle' | 'running' | 'merging' | 'error' | 'complete';

interface Props {
  state: BrainState;
  size?: number;
}

const stateConfig: Record<BrainState, { color: string; glowIntensity: number; pulseSpeed: string }> = {
  idle: { color: '#0DF170', glowIntensity: 0.3, pulseSpeed: '4s' },
  running: { color: '#0DF170', glowIntensity: 0.8, pulseSpeed: '2s' },
  merging: { color: '#00D4FF', glowIntensity: 0.6, pulseSpeed: '1.5s' },
  error: { color: '#EF4444', glowIntensity: 0.7, pulseSpeed: '1s' },
  complete: { color: '#0DF170', glowIntensity: 1.0, pulseSpeed: '3s' },
};

export function NeonBrain({ state, size = 120 }: Props) {
  const cfg = stateConfig[state];
  const r = size / 2;
  const cx = r;
  const cy = r;

  // Generate neural network paths inside the brain
  const neuralPaths = useMemo(() => {
    const paths: string[] = [];
    const nodeCount = 12;
    const nodes: [number, number][] = [];

    for (let i = 0; i < nodeCount; i++) {
      const angle = (i / nodeCount) * Math.PI * 2 + Math.random() * 0.3;
      const dist = r * (0.2 + Math.random() * 0.45);
      nodes.push([cx + Math.cos(angle) * dist, cy + Math.sin(angle) * dist]);
    }

    // Connect nearby nodes
    for (let i = 0; i < nodes.length; i++) {
      for (let j = i + 1; j < nodes.length; j++) {
        const dx = nodes[i][0] - nodes[j][0];
        const dy = nodes[i][1] - nodes[j][1];
        const dist = Math.sqrt(dx * dx + dy * dy);
        if (dist < r * 0.7) {
          paths.push(`M${nodes[i][0]},${nodes[i][1]} L${nodes[j][0]},${nodes[j][1]}`);
        }
      }
    }
    return { paths, nodes };
  }, [r, cx, cy]);

  return (
    <div className="relative" style={{ width: size, height: size }}>
      <svg
        width={size}
        height={size}
        viewBox={`0 0 ${size} ${size}`}
        className="absolute inset-0"
      >
        <defs>
          {/* Glow filter */}
          <filter id="brain-glow" x="-50%" y="-50%" width="200%" height="200%">
            <feGaussianBlur stdDeviation={4 * cfg.glowIntensity} result="blur" />
            <feMerge>
              <feMergeNode in="blur" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>

          {/* Particle glow for running state */}
          <filter id="particle-glow" x="-100%" y="-100%" width="300%" height="300%">
            <feGaussianBlur stdDeviation="3" />
          </filter>

          {/* Radial gradient for brain fill */}
          <radialGradient id="brain-fill" cx="50%" cy="50%" r="50%">
            <stop offset="0%" stopColor={cfg.color} stopOpacity={0.15 * cfg.glowIntensity} />
            <stop offset="70%" stopColor={cfg.color} stopOpacity={0.05} />
            <stop offset="100%" stopColor="transparent" />
          </radialGradient>
        </defs>

        {/* Background circle */}
        <circle
          cx={cx} cy={cy} r={r * 0.85}
          fill="url(#brain-fill)"
          stroke={cfg.color}
          strokeWidth={1.5}
          strokeOpacity={0.15}
        />

        {/* Pulse animation group */}
        <g filter="url(#brain-glow)">
          {/* Main brain outline */}
          <circle
            cx={cx} cy={cy} r={r * 0.65}
            fill="none"
            stroke={cfg.color}
            strokeWidth={2}
            strokeOpacity={cfg.glowIntensity}
          >
            <animate
              attributeName="r"
              values={`${r * 0.62};${r * 0.68};${r * 0.62}`}
              dur={cfg.pulseSpeed}
              repeatCount="indefinite"
            />
            <animate
              attributeName="stroke-opacity"
              values={`${cfg.glowIntensity * 0.6};${cfg.glowIntensity};${cfg.glowIntensity * 0.6}`}
              dur={cfg.pulseSpeed}
              repeatCount="indefinite"
            />
          </circle>

          {/* Inner brain shape — cerebrum lobes approximation */}
          <path
            d={`M${cx},${cy - r * 0.5}
                C${cx + r * 0.45},${cy - r * 0.5} ${cx + r * 0.5},${cy - r * 0.1} ${cx + r * 0.4},${cy + r * 0.1}
                C${cx + r * 0.35},${cy + r * 0.3} ${cx + r * 0.2},${cy + r * 0.45} ${cx},${cy + r * 0.45}
                C${cx - r * 0.2},${cy + r * 0.45} ${cx - r * 0.35},${cy + r * 0.3} ${cx - r * 0.4},${cy + r * 0.1}
                C${cx - r * 0.5},${cy - r * 0.1} ${cx - r * 0.45},${cy - r * 0.5} ${cx},${cy - r * 0.5} Z`}
            fill="none"
            stroke={cfg.color}
            strokeWidth={1.5}
            strokeOpacity={cfg.glowIntensity * 0.7}
          />

          {/* Central fissure */}
          <line
            x1={cx} y1={cy - r * 0.45}
            x2={cx} y2={cy + r * 0.4}
            stroke={cfg.color}
            strokeWidth={1}
            strokeOpacity={cfg.glowIntensity * 0.4}
          />

          {/* Neural network lines */}
          {neuralPaths.paths.map((d, i) => (
            <path
              key={i}
              d={d}
              fill="none"
              stroke={cfg.color}
              strokeWidth={0.5}
              strokeOpacity={cfg.glowIntensity * 0.3}
            />
          ))}

          {/* Neural nodes */}
          {neuralPaths.nodes.map(([nx, ny], i) => (
            <circle
              key={`n${i}`}
              cx={nx} cy={ny} r={1.5}
              fill={cfg.color}
              fillOpacity={cfg.glowIntensity * 0.5}
            >
              {state === 'running' && (
                <animate
                  attributeName="fill-opacity"
                  values={`${cfg.glowIntensity * 0.2};${cfg.glowIntensity * 0.8};${cfg.glowIntensity * 0.2}`}
                  dur={`${1 + (i % 3) * 0.5}s`}
                  repeatCount="indefinite"
                />
              )}
            </circle>
          ))}
        </g>

        {/* Particle ring for running state */}
        {state === 'running' && (
          <g filter="url(#particle-glow)">
            {[0, 60, 120, 180, 240, 300].map((angle, i) => {
              const rad = (angle * Math.PI) / 180;
              return (
                <circle
                  key={`p${i}`}
                  r={2}
                  fill={cfg.color}
                  fillOpacity={0.6}
                >
                  <animateMotion
                    dur={`${3 + i * 0.3}s`}
                    repeatCount="indefinite"
                    path={`M${cx + Math.cos(rad) * r * 0.72},${cy + Math.sin(rad) * r * 0.72} A${r * 0.72},${r * 0.72} 0 1 1 ${cx + Math.cos(rad + 0.01) * r * 0.72},${cy + Math.sin(rad + 0.01) * r * 0.72}`}
                  />
                </circle>
              );
            })}
          </g>
        )}

        {/* Core pulsing dot */}
        <circle
          cx={cx} cy={cy} r={3}
          fill={cfg.color}
          fillOpacity={cfg.glowIntensity}
        >
          <animate
            attributeName="r"
            values="2;4;2"
            dur={cfg.pulseSpeed}
            repeatCount="indefinite"
          />
        </circle>
      </svg>

      {/* Zzz badge for idle state */}
      {state === 'idle' && (
        <div className="absolute -top-1 -right-1 text-xs font-mono text-gray-500 animate-pulse">
          Zzz
        </div>
      )}
    </div>
  );
}
