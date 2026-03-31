import { useEffect, useState } from 'react';
import { useAppStore } from '../stores/appStore';
import * as api from '../services/api';
import type { Suite } from '../models';

export function SuiteGlowBorder({ children }: { children: React.ReactNode }) {
  const { activeSuiteId } = useAppStore();
  const [suite, setSuite] = useState<Suite | null>(null);

  useEffect(() => {
    api.getSuite(activeSuiteId).then(s => setSuite(s)).catch(() => {});
  }, [activeSuiteId]);

  const accentHex = suite?.accentHex ?? '#00E6FF';
  const glowHex = suite?.glowHex ?? '#0099CC';

  return (
    <div className="relative h-screen w-screen overflow-hidden">
      {/* Animated glow border */}
      <div
        className="absolute inset-0 pointer-events-none z-50 suite-glow-border"
        style={{
          '--suite-accent': accentHex,
          '--suite-glow': glowHex,
        } as React.CSSProperties}
      />
      {/* App content */}
      <div className="relative h-full w-full">
        {children}
      </div>

      <style>{`
        .suite-glow-border {
          box-shadow:
            inset 0 0 1px 0 var(--suite-accent),
            inset 0 0 4px -1px var(--suite-glow);
          border-radius: 0;
          animation: suite-pulse 4s ease-in-out infinite;
        }

        @keyframes suite-pulse {
          0%, 100% {
            box-shadow:
              inset 0 0 1px 0 var(--suite-accent),
              inset 0 0 4px -1px var(--suite-glow);
            opacity: 0.6;
          }
          50% {
            box-shadow:
              inset 0 0 2px 0 var(--suite-accent),
              inset 0 0 8px -1px var(--suite-glow);
            opacity: 1;
          }
        }
      `}</style>
    </div>
  );
}
