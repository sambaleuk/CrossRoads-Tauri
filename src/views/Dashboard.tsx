import { useRef, useState, useEffect, useCallback } from 'react';
import { useAppStore } from '../stores/appStore';
import { listen } from '@tauri-apps/api/event';
import { TerminalSlot } from '../components/TerminalSlot';
import { SlotTerminal } from '../components/SlotTerminal';
import { SlotConfigDialog } from '../components/SlotConfigDialog';
import { NeonBrain } from '../components/NeonBrain';
import { SynapseConnections } from '../components/SynapseConnections';
import { DashboardMetrics } from '../components/DashboardMetrics';
import * as api from '../services/api';
import type { AgentSlot, AgentSlotStatus } from '../models';

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

// Hexagonal slot positioning: 6 slots around center
// Slot 0=top, 1=top-right, 2=bottom-right, 3=bottom, 4=bottom-left, 5=top-left
const HEX_ANGLES = [270, 330, 30, 90, 150, 210]; // degrees, starting from top

function computeHexPositions(containerW: number, containerH: number, slotW: number, slotH: number) {
  const cx = containerW / 2;
  const cy = containerH / 2;
  // Radius from center to slot center — adaptive to container size
  const rx = Math.min(containerW * 0.35, 320);
  const ry = Math.min(containerH * 0.35, 260);

  return HEX_ANGLES.map((deg) => {
    const rad = (deg * Math.PI) / 180;
    return {
      left: cx + Math.cos(rad) * rx - slotW / 2,
      top: cy + Math.sin(rad) * ry - slotH / 2,
      // Connection point (center of slot card)
      cx: cx + Math.cos(rad) * rx,
      cy: cy + Math.sin(rad) * ry,
    };
  });
}

function deriveBrainState(slots: AgentSlot[]): 'idle' | 'running' | 'merging' | 'error' | 'complete' {
  const statuses = slots.map((s) => s.status);
  if (statuses.some((s) => s === 'error')) return 'error';
  if (statuses.some((s) => s === 'running' || s === 'provisioning')) return 'running';
  if (statuses.every((s) => s === 'done')) return 'complete';
  return 'idle';
}

export function Dashboard() {
  const { slots: activeSlots, slotCosts, expandedSlotId, setExpandedSlotId, session } = useAppStore();
  const displaySlots = activeSlots.length > 0 ? activeSlots : defaultSlots;
  const [configSlotIndex, setConfigSlotIndex] = useState<number | null>(null);

  // Listen for agent-status events to refresh slots in real-time
  useEffect(() => {
    const unlisteners: (() => void)[] = [];

    listen<{ slotId: string; status: string }>('agent-status', async () => {
      // Refresh all slots from DB when any slot status changes
      if (session?.id) {
        const freshSlots = await api.fetchSlots(session.id);
        useAppStore.getState().setSlots(freshSlots);
      }
    }).then(u => unlisteners.push(u));

    // Also listen for log entries (slots launching produces these)
    listen('log-entry', async () => {
      if (session?.id && activeSlots.some(s => s.status === 'provisioning' || s.status === 'empty')) {
        const freshSlots = await api.fetchSlots(session.id);
        useAppStore.getState().setSlots(freshSlots);
      }
    }).then(u => unlisteners.push(u));

    return () => { unlisteners.forEach(u => u()); };
  }, [session?.id]);

  const containerRef = useRef<HTMLDivElement>(null);
  const [dims, setDims] = useState({ w: 900, h: 700 });

  const updateDims = useCallback(() => {
    if (containerRef.current) {
      setDims({
        w: containerRef.current.offsetWidth,
        h: containerRef.current.offsetHeight,
      });
    }
  }, []);

  useEffect(() => {
    updateDims();
    window.addEventListener('resize', updateDims);
    return () => window.removeEventListener('resize', updateDims);
  }, [updateDims]);

  const slotW = Math.min(220, dims.w * 0.22);
  const slotH = Math.min(200, dims.h * 0.25);
  const positions = computeHexPositions(dims.w, dims.h, slotW, slotH);

  const brainState = deriveBrainState(displaySlots);
  const brainSize = Math.min(120, dims.w * 0.12, dims.h * 0.16);

  const slotInfos = displaySlots.map((s) => ({
    status: s.status as AgentSlotStatus,
    agentType: s.agentType,
  }));

  const connectionPoints = positions.map((p) => ({ x: p.cx, y: p.cy }));

  // US-004: Single terminal expanded mode
  const expandedSlot = expandedSlotId
    ? displaySlots.find((s) => s.id === expandedSlotId)
    : null;

  if (expandedSlot) {
    return (
      <div ref={containerRef} className="flex-1 overflow-hidden relative flex flex-col">
        {/* Header bar */}
        <div className="flex items-center gap-3 px-4 py-2 border-b border-gray-800 bg-gray-900/80">
          <button
            onClick={() => setExpandedSlotId(null)}
            className="text-xs font-mono text-gray-400 hover:text-neon-green transition-colors"
          >
            ← Grid
          </button>
          <span className="text-xs font-bold font-mono text-gray-300">
            SLOT #{expandedSlot.slotIndex + 1}
          </span>
          <span className="text-[10px] font-mono text-gray-500">
            {expandedSlot.agentType} — {expandedSlot.branchName ?? 'no branch'}
          </span>
        </div>
        {/* Full-size terminal */}
        <div className="flex-1">
          <SlotTerminal slotId={expandedSlot.id} isExpanded />
        </div>
      </div>
    );
  }

  return (
    <div ref={containerRef} className="flex-1 overflow-hidden relative">
      {/* Synapse connections (SVG underlay) */}
      <SynapseConnections
        centerX={dims.w / 2}
        centerY={dims.h / 2}
        slotPositions={connectionPoints}
        slots={slotInfos}
        width={dims.w}
        height={dims.h}
      />

      {/* NeonBrain at center */}
      <div
        className="absolute z-10"
        style={{
          left: dims.w / 2 - brainSize / 2,
          top: dims.h / 2 - brainSize / 2,
        }}
      >
        <NeonBrain state={brainState} size={brainSize} />
      </div>

      {/* Slot config dialog */}
      {configSlotIndex !== null && (
        <SlotConfigDialog
          slotIndex={configSlotIndex}
          branches={['main', 'feat/backend', 'feat/frontend', 'feat/testing']}
          onConfirm={(config) => {
            console.log('Slot configured:', configSlotIndex, config);
            setConfigSlotIndex(null);
          }}
          onClose={() => setConfigSlotIndex(null)}
        />
      )}

      {/* 6 Hexagonal slots */}
      {displaySlots.slice(0, 6).map((slot, i) => {
        const pos = positions[i];
        if (!pos) return null;
        return (
          <div
            key={slot.id}
            className="absolute z-10 transition-all duration-300"
            style={{
              left: pos.left,
              top: pos.top,
              width: slotW,
            }}
          >
            <TerminalSlot
              slot={slot}
              cost={slotCosts[slot.id]}
              isConfigured={slot.status !== 'empty'}
              onConfigure={() => setConfigSlotIndex(slot.slotIndex)}
            />
          </div>
        );
      })}

      {/* Live metrics bar */}
      <DashboardMetrics />
    </div>
  );
}
