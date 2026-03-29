import { useState, useRef, useEffect } from 'react';
import { useAppStore } from '../stores/appStore';

type MessageRole = 'user' | 'assistant' | 'system' | 'error';
type ApiMode = 'cli' | 'api';

interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  timestamp: string;
}

const SUGGESTIONS = [
  'Start orchestration',
  'Show agent status',
  'Load PRD',
  'Pause all agents',
  'Show costs',
];

const roleStyles: Record<MessageRole, string> = {
  user: 'text-neon-green',
  assistant: 'text-gray-300',
  system: 'text-gray-500 italic',
  error: 'text-red-400',
};

const roleLabels: Record<MessageRole, string> = {
  user: 'YOU',
  assistant: 'ORCH',
  system: 'SYS',
  error: 'ERR',
};

export function ChatPanel() {
  const { session, projectPath } = useAppStore();
  const [messages, setMessages] = useState<ChatMessage[]>([
    { id: '0', role: 'system', content: 'XRoads orchestrator ready. Ask about features, PRDs, or agent status.', timestamp: new Date().toISOString() },
  ]);
  const [input, setInput] = useState('');
  const [apiMode, setApiMode] = useState<ApiMode>('cli');
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const sendMessage = () => {
    if (!input.trim()) return;
    const userMsg: ChatMessage = {
      id: Date.now().toString(),
      role: 'user',
      content: input.trim(),
      timestamp: new Date().toISOString(),
    };
    setMessages((prev) => [...prev, userMsg]);
    setInput('');

    setTimeout(() => {
      setMessages((prev) => [...prev, {
        id: (Date.now() + 1).toString(),
        role: 'assistant',
        content: `Received: "${userMsg.content}". Processing via ${apiMode} mode...`,
        timestamp: new Date().toISOString(),
      }]);
    }, 300);
  };

  return (
    <div className="w-72 flex flex-col border-r border-gray-800 bg-gray-950">
      {/* Header */}
      <div className="px-3 py-2 border-b border-gray-800 flex items-center justify-between">
        <span className="text-[11px] font-bold font-mono text-gray-300 tracking-wider">ORCHESTRATOR</span>
        <div className="flex gap-1">
          {(['cli', 'api'] as const).map((mode) => (
            <button
              key={mode}
              onClick={() => setApiMode(mode)}
              className={`px-1.5 py-0.5 text-[9px] font-mono rounded transition-colors ${
                apiMode === mode ? 'bg-neon-green/20 text-neon-green' : 'text-gray-500 hover:text-gray-400'
              }`}
            >
              {mode.toUpperCase()}
            </button>
          ))}
        </div>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-auto p-3 space-y-3">
        {messages.map((msg) => (
          <div key={msg.id} className="space-y-0.5">
            <div className="flex items-center gap-1.5">
              <span className={`text-[9px] font-bold font-mono ${roleStyles[msg.role]}`}>
                {roleLabels[msg.role]}
              </span>
              <span className="text-[8px] text-gray-600 font-mono">
                {new Date(msg.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
              </span>
            </div>
            <div className={`text-[11px] font-mono leading-relaxed ${roleStyles[msg.role]}`}>
              {msg.content}
            </div>
          </div>
        ))}
        <div ref={bottomRef} />
      </div>

      {/* Suggestion chips */}
      <div className="px-3 py-1.5 flex flex-wrap gap-1 border-t border-gray-800/50">
        {SUGGESTIONS.map((s) => (
          <button
            key={s}
            onClick={() => setInput(s)}
            className="px-2 py-0.5 text-[9px] font-mono text-gray-500 bg-gray-800/50 rounded hover:text-neon-green hover:bg-gray-800 transition-colors"
          >
            {s}
          </button>
        ))}
      </div>

      {/* Input */}
      <div className="p-2 border-t border-gray-800">
        <div className="flex gap-1.5">
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && sendMessage()}
            placeholder="Ask about features, PRDs..."
            className="flex-1 bg-gray-900 border border-gray-700 rounded px-2 py-1.5 text-[11px] font-mono text-gray-200 placeholder:text-gray-600 focus:border-neon-green/40 focus:outline-none"
          />
          <button
            onClick={sendMessage}
            className="px-2.5 py-1.5 bg-neon-green/20 text-neon-green text-[10px] font-mono rounded border border-neon-green/30 hover:bg-neon-green/30 transition-colors"
          >
            ↵
          </button>
        </div>
      </div>

      {/* Status bar */}
      <div className="px-3 py-1 border-t border-gray-800 flex items-center gap-2 text-[8px] font-mono text-gray-600">
        <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
        <span className="truncate">{projectPath?.split('/').pop() ?? 'No project'}</span>
        <span>|</span>
        <span>{session?.status ?? 'idle'}</span>
        <span>|</span>
        <span>{apiMode}</span>
      </div>
    </div>
  );
}
