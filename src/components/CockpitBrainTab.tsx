import { useState, useEffect, useRef, useCallback } from 'react';
import { useAppStore } from '../stores/appStore';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import * as api from '../services/api';

// ── PRD-44: Live Consciousness Stream ──
// The Brain tab renders the cockpit's live stream of thinking, decisions, and actions.

interface CockpitEvent {
  id: string;
  eventType: 'thinking' | 'action' | 'decision' | 'loop' | 'subagent' | 'status' | 'report' | 'log' | 'error';
  content: string;
  timestamp: string;
  metadata?: any;
}

interface CockpitEventPayload {
  eventType: string;
  content: string;
  timestamp: string;
  metadata?: any;
}

interface CockpitStatus {
  isAlive: boolean;
  processId?: number;
  sessionId?: string;
  startedAt?: string;
  eventCount: number;
}

const MAX_ENTRIES = 200;

export function CockpitBrainTab() {
  const { projectPath } = useAppStore();
  const [events, setEvents] = useState<CockpitEvent[]>([]);
  const [status, setStatus] = useState<CockpitStatus | null>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const feedRef = useRef<HTMLDivElement>(null);
  const eventIdRef = useRef(0);

  // Auto-scroll to bottom when new events arrive
  useEffect(() => {
    if (autoScroll && feedRef.current) {
      feedRef.current.scrollTop = feedRef.current.scrollHeight;
    }
  }, [events, autoScroll]);

  // Subscribe to cockpit events
  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];

    const eventTypes = [
      'cockpit-thinking',
      'cockpit-action',
      'cockpit-decision',
      'cockpit-loop',
      'cockpit-subagent',
      'cockpit-status',
      'cockpit-report',
      'cockpit-log',
      'cockpit-error',
    ];

    for (const eventName of eventTypes) {
      listen<CockpitEventPayload>(eventName, (event) => {
        const newEvent: CockpitEvent = {
          id: `ce-${++eventIdRef.current}`,
          eventType: event.payload.eventType as CockpitEvent['eventType'],
          content: event.payload.content,
          timestamp: event.payload.timestamp,
          metadata: event.payload.metadata,
        };
        setEvents(prev => {
          const updated = [...prev, newEvent];
          return updated.length > MAX_ENTRIES ? updated.slice(-MAX_ENTRIES) : updated;
        });
      }).then(unlisten => unlisteners.push(unlisten));
    }

    return () => {
      unlisteners.forEach(fn => fn());
    };
  }, []);

  // Poll cockpit status
  useEffect(() => {
    const poll = async () => {
      try {
        const s = await api.getCockpitStatus();
        setStatus(s);
      } catch { /* not available yet */ }
    };
    poll();
    const interval = setInterval(poll, 5000);
    return () => clearInterval(interval);
  }, []);

  const handleStart = useCallback(async () => {
    if (!projectPath) return;
    try {
      await api.startCockpitSession(projectPath);
    } catch { /* error handled by events */ }
  }, [projectPath]);

  const handleStop = useCallback(async () => {
    try {
      await api.stopCockpitSession();
    } catch { /* error */ }
  }, []);

  // Detect scroll position for auto-scroll toggle
  const handleScroll = () => {
    if (!feedRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = feedRef.current;
    setAutoScroll(scrollHeight - scrollTop - clientHeight < 50);
  };

  const isAlive = status?.isAlive ?? false;

  // Empty state — cockpit not started
  if (!isAlive && events.length === 0) {
    return (
      <div className="flex items-center justify-center h-full p-8">
        <div className="text-center space-y-3">
          <div className="relative">
            <div className="w-12 h-12 mx-auto rounded-full bg-gray-800 flex items-center justify-center">
              <div className="w-6 h-6 rounded-full bg-gray-700" />
            </div>
          </div>
          <div className="text-[10px] text-gray-500 font-mono">Cockpit brain offline</div>
          <button
            onClick={handleStart}
            disabled={!projectPath}
            className="text-[9px] font-mono px-3 py-1.5 rounded bg-neon-green/10 text-neon-green hover:bg-neon-green/20 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
          >
            Start Cockpit Brain
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center gap-2 px-3 py-1.5 border-b border-gray-800/50">
        <span className={`w-2 h-2 rounded-full ${isAlive ? 'bg-neon-green animate-pulse shadow-neon-green/50 shadow-sm' : 'bg-gray-600'}`} />
        <span className="text-[9px] font-mono text-gray-400">
          {isAlive ? 'ALIVE' : 'OFFLINE'}
        </span>
        {status?.eventCount !== undefined && (
          <span className="text-[8px] font-mono text-gray-600">{status.eventCount} events</span>
        )}
        <div className="flex-1" />
        {isAlive ? (
          <button onClick={handleStop} className="text-[8px] font-mono text-red-400 hover:bg-red-400/10 px-1.5 py-0.5 rounded">
            Stop
          </button>
        ) : (
          <button onClick={handleStart} className="text-[8px] font-mono text-neon-green hover:bg-neon-green/10 px-1.5 py-0.5 rounded">
            Start
          </button>
        )}
      </div>

      {/* Starting state */}
      {isAlive && events.length === 0 && (
        <div className="flex items-center justify-center p-8">
          <div className="text-center space-y-2">
            <div className="w-8 h-8 mx-auto rounded-full bg-neon-green/10 flex items-center justify-center animate-pulse">
              <div className="w-4 h-4 rounded-full bg-neon-green/30" />
            </div>
            <div className="text-[9px] text-gray-500 font-mono">Cockpit brain starting...</div>
          </div>
        </div>
      )}

      {/* Event feed */}
      <div ref={feedRef} onScroll={handleScroll} className="flex-1 overflow-auto p-2 space-y-0.5">
        {events.map(event => (
          <EventEntry key={event.id} event={event} />
        ))}
      </div>

      {/* Auto-scroll indicator */}
      {!autoScroll && events.length > 10 && (
        <button
          onClick={() => { setAutoScroll(true); feedRef.current?.scrollTo({ top: feedRef.current.scrollHeight, behavior: 'smooth' }); }}
          className="absolute bottom-2 right-2 text-[8px] font-mono text-gray-400 bg-gray-800 px-2 py-1 rounded hover:bg-gray-700"
        >
          Auto-scroll
        </button>
      )}
    </div>
  );
}

function EventEntry({ event }: { event: CockpitEvent }) {
  const time = new Date(event.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });

  switch (event.eventType) {
    case 'thinking':
      return (
        <div className="flex gap-1.5 py-0.5">
          <span className="text-[7px] font-mono text-gray-600 shrink-0 w-14">{time}</span>
          <span className="text-[9px] font-mono text-gray-500 italic leading-relaxed">{event.content}</span>
        </div>
      );

    case 'action':
      return (
        <div className="flex items-center gap-1.5 py-0.5">
          <span className="text-[7px] font-mono text-gray-600 shrink-0 w-14">{time}</span>
          <span className="text-[8px] font-mono text-cyan-400 bg-cyan-400/10 px-1.5 py-0.5 rounded">{event.content}</span>
        </div>
      );

    case 'decision':
      return (
        <div className="flex gap-1.5 py-1 border-l-2 border-neon-green/40 pl-2">
          <span className="text-[7px] font-mono text-gray-600 shrink-0 w-14">{time}</span>
          <span className="text-[9px] font-mono text-neon-green font-bold leading-relaxed">{event.content}</span>
        </div>
      );

    case 'loop':
      return (
        <div className="flex items-center gap-1.5 py-0.5">
          <span className="text-[7px] font-mono text-gray-600 shrink-0 w-14">{time}</span>
          <span className="w-1.5 h-1.5 rounded-full bg-neon-purple animate-pulse" />
          <span className="text-[8px] font-mono text-neon-purple/60">{event.content}</span>
        </div>
      );

    case 'subagent':
      return (
        <div className="flex items-center gap-1.5 py-0.5">
          <span className="text-[7px] font-mono text-gray-600 shrink-0 w-14">{time}</span>
          <span className="text-[8px] font-mono text-amber-400 bg-amber-400/10 px-1.5 py-0.5 rounded">{event.content}</span>
        </div>
      );

    case 'status':
      return (
        <div className="flex gap-1.5 py-0.5 border-l-2 border-cyan-400/40 pl-2">
          <span className="text-[7px] font-mono text-gray-600 shrink-0 w-14">{time}</span>
          <span className="text-[9px] font-mono text-cyan-400 leading-relaxed">{event.content}</span>
        </div>
      );

    case 'report':
      return (
        <div className="flex gap-1.5 py-1 border-l-2 border-amber-400/40 pl-2">
          <span className="text-[7px] font-mono text-gray-600 shrink-0 w-14">{time}</span>
          <span className="text-[8px] shrink-0">&#128196;</span>
          <span className="text-[9px] font-mono text-amber-300 leading-relaxed">{event.content}</span>
        </div>
      );

    case 'log':
      return (
        <div className="flex gap-1.5 py-0.5">
          <span className="text-[7px] font-mono text-gray-600 shrink-0 w-14">{time}</span>
          <span className="text-[9px] font-mono text-gray-500 leading-relaxed">{event.content}</span>
        </div>
      );

    case 'error':
      return (
        <div className="flex gap-1.5 py-1 border-l-2 border-red-500/60 pl-2 bg-red-500/5 rounded-r">
          <span className="text-[7px] font-mono text-gray-600 shrink-0 w-14">{time}</span>
          <span className="text-[8px] shrink-0">&#9888;</span>
          <span className="text-[9px] font-mono text-red-400 font-bold leading-relaxed">{event.content}</span>
        </div>
      );

    default:
      return (
        <div className="flex gap-1.5 py-0.5">
          <span className="text-[7px] font-mono text-gray-600 shrink-0 w-14">{time}</span>
          <span className="text-[9px] font-mono text-gray-400">{event.content}</span>
        </div>
      );
  }
}
