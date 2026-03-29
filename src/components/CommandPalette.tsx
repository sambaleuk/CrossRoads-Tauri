import { useState, useEffect, useRef, useCallback } from 'react';
import { useAppStore } from '../stores/appStore';

interface Command {
  id: string;
  label: string;
  shortcut?: string;
  category: string;
  action: () => void;
}

export function useCommandPalette() {
  const [isOpen, setIsOpen] = useState(false);
  const open = useCallback(() => setIsOpen(true), []);
  const close = useCallback(() => setIsOpen(false), []);

  // Ctrl+K to open
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setIsOpen((prev) => !prev);
      }
      if (e.key === 'Escape') setIsOpen(false);
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);

  return { isOpen, open, close };
}

interface Props {
  onClose: () => void;
}

export function CommandPalette({ onClose }: Props) {
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const store = useAppStore();

  const commands: Command[] = [
    { id: 'toggle-cockpit', label: 'Toggle Cockpit Panel', shortcut: '⌘⇧C', category: 'View', action: () => store.toggleCockpit() },
    { id: 'toggle-inspector', label: 'Toggle Inspector Panel', shortcut: '⌘⇧I', category: 'View', action: () => store.toggleInspector() },
    { id: 'toggle-chat', label: 'Toggle Chat Panel', shortcut: '⌘⇧O', category: 'View', action: () => store.toggleChat() },
    { id: 'collapse-expanded', label: 'Exit Expanded Terminal', category: 'View', action: () => store.setExpandedSlotId(null) },
    { id: 'new-session', label: 'New Session', category: 'Session', action: () => {} },
    { id: 'pause-session', label: 'Pause Session', category: 'Session', action: () => {} },
    { id: 'resume-session', label: 'Resume Session', category: 'Session', action: () => {} },
    { id: 'close-session', label: 'Close Session', category: 'Session', action: () => {} },
    { id: 'load-prd', label: 'Load PRD File', shortcut: '⌘L', category: 'Orchestration', action: () => {} },
    { id: 'start-orchestration', label: 'Start Orchestration', category: 'Orchestration', action: () => {} },
    { id: 'stop-all', label: 'Stop All Agents', category: 'Agent', action: () => {} },
    { id: 'health-check', label: 'Run Health Check', category: 'Agent', action: () => {} },
  ];

  const filtered = query
    ? commands.filter((c) =>
        c.label.toLowerCase().includes(query.toLowerCase()) ||
        c.category.toLowerCase().includes(query.toLowerCase())
      )
    : commands;

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  const execute = (cmd: Command) => {
    cmd.action();
    onClose();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((i) => Math.min(i + 1, filtered.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === 'Enter' && filtered[selectedIndex]) {
      execute(filtered[selectedIndex]);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh] bg-black/50" onClick={onClose}>
      <div
        className="w-[500px] bg-gray-900 border border-gray-700 rounded-xl shadow-2xl overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Search input */}
        <div className="px-4 py-3 border-b border-gray-800">
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Type a command..."
            className="w-full bg-transparent text-sm font-mono text-gray-200 placeholder:text-gray-500 focus:outline-none"
          />
        </div>

        {/* Results */}
        <div className="max-h-[300px] overflow-auto py-1">
          {filtered.length === 0 ? (
            <div className="px-4 py-6 text-center text-[11px] font-mono text-gray-500">
              No commands found
            </div>
          ) : (
            filtered.map((cmd, i) => (
              <button
                key={cmd.id}
                onClick={() => execute(cmd)}
                className={`w-full flex items-center justify-between px-4 py-2 text-left transition-colors ${
                  i === selectedIndex ? 'bg-neon-green/10 text-neon-green' : 'text-gray-300 hover:bg-gray-800'
                }`}
              >
                <div className="flex items-center gap-3">
                  <span className="text-[9px] font-mono text-gray-500 w-20">{cmd.category}</span>
                  <span className="text-[12px] font-mono">{cmd.label}</span>
                </div>
                {cmd.shortcut && (
                  <span className="text-[10px] font-mono text-gray-500">{cmd.shortcut}</span>
                )}
              </button>
            ))
          )}
        </div>

        {/* Footer */}
        <div className="px-4 py-2 border-t border-gray-800 flex items-center gap-4 text-[9px] font-mono text-gray-600">
          <span>↑↓ Navigate</span>
          <span>↵ Execute</span>
          <span>Esc Close</span>
        </div>
      </div>
    </div>
  );
}
