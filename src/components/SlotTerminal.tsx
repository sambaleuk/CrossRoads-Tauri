import { useRef, useEffect } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import type { PtyOutputPayload, LogEntryPayload } from '../models';
import '@xterm/xterm/css/xterm.css';

interface Props {
  slotId: string;
  isExpanded?: boolean;
}

const THEME = {
  background: '#0a0a0f',
  foreground: '#e4e4e7',
  cursor: '#0DF170',
  cursorAccent: '#0a0a0f',
  selectionBackground: '#0DF17030',
  selectionForeground: '#ffffff',
  black: '#1a1a2e',
  red: '#EF4444',
  green: '#0DF170',
  yellow: '#FBBF24',
  blue: '#00D4FF',
  magenta: '#8B5CF6',
  cyan: '#22D3EE',
  white: '#e4e4e7',
  brightBlack: '#4a4a5e',
  brightRed: '#F87171',
  brightGreen: '#34D399',
  brightYellow: '#FCD34D',
  brightBlue: '#60A5FA',
  brightMagenta: '#A78BFA',
  brightCyan: '#67E8F9',
  brightWhite: '#ffffff',
};

export function SlotTerminal({ slotId, isExpanded = false }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;

    const term = new Terminal({
      theme: THEME,
      fontFamily: "'JetBrains Mono', 'SF Mono', 'Fira Code', monospace",
      fontSize: isExpanded ? 13 : 11,
      lineHeight: 1.2,
      cursorBlink: true,
      cursorStyle: 'bar',
      scrollback: 5000,
      allowProposedApi: true,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);

    term.open(containerRef.current);
    fitAddon.fit();

    termRef.current = term;
    fitAddonRef.current = fitAddon;

    // Subscribe to PTY output events filtered by slotId
    let mounted = true;
    const unlisteners: UnlistenFn[] = [];

    listen<PtyOutputPayload>('pty-output', (event) => {
      if (mounted && event.payload.slotId === slotId) {
        term.write(event.payload.text);
      }
    }).then((unlisten) => {
      if (mounted) unlisteners.push(unlisten); else unlisten();
    });

    // Subscribe to structured events from headless launcher (PRD-43)
    listen<LogEntryPayload>('log-entry', (event) => {
      if (!mounted) return;
      const { source, message, slotId: eventSlotId } = event.payload;
      if (eventSlotId !== slotId) return;

      switch (source) {
        case 'agent-text':
          term.write(message);
          break;
        case 'agent-tool':
          // Tool calls rendered as colored badges
          term.write(`\r\n\x1b[36m${message}\x1b[0m\r\n`);
          break;
        case 'agent-tool-result':
          term.write(`\x1b[90m${message.slice(0, 200)}\x1b[0m\r\n`);
          break;
        case 'agent-error':
          term.write(`\r\n\x1b[31m[ERROR] ${message}\x1b[0m\r\n`);
          break;
        case 'agent-session':
          term.write(`\r\n\x1b[32m${message}\x1b[0m\r\n`);
          break;
      }
    }).then((unlisten) => {
      if (mounted) unlisteners.push(unlisten); else unlisten();
    });

    // US-003: Capture keyboard input and send to PTY
    term.onData((data) => {
      invoke('send_pty_input', { slotId, text: data }).catch(() => {
        // PTY may not be active yet — silently ignore
      });
    });

    // Resize handler
    const resizeObserver = new ResizeObserver(() => {
      if (fitAddonRef.current) {
        try {
          fitAddonRef.current.fit();
        } catch {
          // Container might be hidden
        }
      }
    });
    resizeObserver.observe(containerRef.current);

    return () => {
      mounted = false;
      resizeObserver.disconnect();
      unlisteners.forEach(fn => fn());
      if (unlistenRef.current) {
        unlistenRef.current();
      }
      term.dispose();
      termRef.current = null;
      fitAddonRef.current = null;
    };
  }, [slotId, isExpanded]);

  return (
    <div
      ref={containerRef}
      className="w-full h-full min-h-[80px]"
      style={{ backgroundColor: '#0a0a0f' }}
    />
  );
}
