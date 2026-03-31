import { useState, useEffect, useRef } from 'react';
import { useAppStore } from '../stores/appStore';
import { emit } from '@tauri-apps/api/event';
import * as api from '../services/api';
import type { Suite } from '../models';

// Icon mapping: SF Symbol names to emoji/text equivalents for web
const suiteIcons: Record<string, string> = {
  'chevron.left.forwardslash.chevron.right': '</>', // developer
  'megaphone.fill': '\u{1F4E3}',                    // marketer
  'magnifyingglass.circle.fill': '\u{1F50D}',       // researcher
  'building.2.fill': '\u{1F3E2}',                   // ops
};

export function SuiteSwitcher() {
  const { activeSuiteId, setActiveSuiteId } = useAppStore();
  const [suites, setSuites] = useState<Suite[]>([]);
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    api.listSuites().then(setSuites).catch(() => {});
  }, []);

  // Close dropdown on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    if (open) document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [open]);

  const active = suites.find(s => s.id === activeSuiteId);
  const icon = active ? (suiteIcons[active.icon] ?? active.icon) : '</>';
  const accentHex = active?.accentHex ?? '#00E6FF';

  const handleSwitch = async (suiteId: string) => {
    setActiveSuiteId(suiteId);
    setOpen(false);
    try {
      await emit('suite-switched', { suiteId });
    } catch { /* event bus not ready */ }
  };

  return (
    <div ref={ref} className="relative">
      <button
        onClick={() => setOpen(!open)}
        className="flex items-center gap-1.5 px-2 py-1 rounded text-[10px] font-mono transition-colors hover:bg-gray-800"
        style={{ color: accentHex, borderColor: `${accentHex}33`, borderWidth: '1px' }}
        title="Switch Suite"
      >
        <span className="text-xs">{icon}</span>
        <span>{active?.name ?? 'Developer'}</span>
        <span className="text-[8px] text-gray-500">{'\u25BE'}</span>
      </button>

      {open && (
        <div className="absolute top-full left-0 mt-1 w-64 bg-gray-900 border border-gray-700 rounded-lg shadow-xl z-50 overflow-hidden">
          {suites.map(suite => {
            const isActive = suite.id === activeSuiteId;
            const sIcon = suiteIcons[suite.icon] ?? suite.icon;
            return (
              <button
                key={suite.id}
                onClick={() => handleSwitch(suite.id)}
                className={`w-full flex items-center gap-2.5 px-3 py-2 text-left transition-colors ${
                  isActive ? 'bg-gray-800' : 'hover:bg-gray-800/60'
                }`}
              >
                <span className="text-base" style={{ color: suite.accentHex }}>{sIcon}</span>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-1.5">
                    <span className="text-[10px] font-bold font-mono" style={{ color: suite.accentHex }}>
                      {suite.name}
                    </span>
                    {isActive && (
                      <span className="text-[7px] font-mono px-1 py-0.5 rounded"
                        style={{ backgroundColor: `${suite.accentHex}20`, color: suite.accentHex }}>
                        ACTIVE
                      </span>
                    )}
                  </div>
                  <div className="text-[8px] font-mono text-gray-500 truncate">{suite.description}</div>
                </div>
                <div className="flex gap-0.5">
                  <span className="w-1.5 h-1.5 rounded-full" style={{ backgroundColor: suite.accentHex }} />
                  <span className="w-1.5 h-1.5 rounded-full" style={{ backgroundColor: suite.glowHex }} />
                </div>
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
