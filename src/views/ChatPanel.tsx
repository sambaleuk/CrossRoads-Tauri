import { useState } from 'react';

interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: string;
}

export function ChatPanel() {
  const [messages, setMessages] = useState<ChatMessage[]>([
    { id: '1', role: 'system', content: 'XRoads Orchestrator ready. Ask about features, request PRDs, or get help.', timestamp: new Date().toISOString() },
  ]);
  const [input, setInput] = useState('');

  const send = () => {
    if (!input.trim()) return;
    const userMsg: ChatMessage = { id: Date.now().toString(), role: 'user', content: input.trim(), timestamp: new Date().toISOString() };
    setMessages((prev) => [...prev, userMsg]);
    setInput('');

    // Mock assistant response
    setTimeout(() => {
      const reply: ChatMessage = {
        id: (Date.now() + 1).toString(), role: 'assistant',
        content: `Received: "${userMsg.content}". Orchestrator integration coming in next PRD.`,
        timestamp: new Date().toISOString(),
      };
      setMessages((prev) => [...prev, reply]);
    }, 300);
  };

  return (
    <div className="flex flex-col h-full">
      <div className="p-3 border-b border-gray-800">
        <h2 className="text-xs font-bold font-mono text-gray-400">ORCHESTRATOR</h2>
      </div>

      <div className="flex-1 overflow-auto p-3 space-y-3">
        {messages.map((msg) => (
          <div key={msg.id} className={`text-[11px] font-mono leading-relaxed ${
            msg.role === 'user' ? 'text-neon-green' : msg.role === 'system' ? 'text-gray-500 italic' : 'text-gray-300'
          }`}>
            <span className="text-[9px] text-gray-600">{msg.role === 'user' ? '>' : msg.role === 'system' ? 'SYS' : 'AI'}</span>{' '}
            {msg.content}
          </div>
        ))}
      </div>

      <div className="p-2 border-t border-gray-800">
        <div className="flex gap-2">
          <input
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && send()}
            placeholder="Ask about features, PRDs..."
            className="flex-1 bg-gray-900 border border-gray-700 rounded-lg px-3 py-1.5 text-[11px] font-mono text-gray-200 focus:outline-none focus:border-neon-green/50"
          />
          <button onClick={send} className="px-3 py-1.5 bg-neon-green/10 text-neon-green rounded-lg text-[11px] font-mono hover:bg-neon-green/20">
            Send
          </button>
        </div>
      </div>
    </div>
  );
}
