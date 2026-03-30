import { useState, useRef, useEffect, useCallback } from 'react';
import { useAppStore } from '../stores/appStore';
import { invoke } from '@tauri-apps/api/core';
import { PRDDetectedOverlay, detectPRDInResponse } from '../components/PRDDetectedOverlay';
import { SlotAssignmentDialog } from '../components/SlotAssignmentDialog';
import { saveChatMessage } from '../services/api';

type MessageRole = 'user' | 'assistant' | 'system' | 'error';
type ChatMode = 'cli' | 'api';

interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  timestamp: string;
  isStreaming?: boolean;
}

const SYSTEM_PROMPT = `You are the XRoads orchestrator assistant — an AI pair-programmer that helps developers manage multi-agent coding sessions.

You have deep knowledge of:
- PRD-driven development (user stories, dependency layers, parallel dispatch)
- Git worktree isolation (each agent works on its own branch)
- Agent orchestration (Claude Code, Gemini CLI, Codex CLI)
- Cost management (budget caps, model routing, cost projections)
- Code safety (dangerous operation detection, approval gates)

CRITICAL: When the user asks you to create a PRD, plan a feature, or build something, you MUST include a JSON code block in your response with this EXACT format:

\`\`\`json
{
  "feature_name": "Feature Name Here",
  "description": "One-line description",
  "status": "pending",
  "user_stories": [
    {
      "id": "US-001",
      "title": "Story title",
      "description": "What to implement",
      "status": "pending",
      "tests": ["test description"],
      "dependencies": [],
      "priority": "high"
    }
  ]
}
\`\`\`

This JSON block triggers XRoads' orchestration pipeline. Without it, the user cannot launch agents.
Priority values: "high", "medium", "low". Dependencies reference other story IDs.
Always include the JSON block PLUS a summary after it.

Be concise and direct. Lead with the answer.`;

const CLI_SUGGESTIONS = ['Create a PRD for...', 'Plan orchestration', 'What agents should I use?', 'Explain my codebase'];
const API_SUGGESTIONS = ['Write a PRD for auth', 'Analyze dependencies', 'How to structure this?', 'Review my approach'];

function loadApiKey(): string {
  try {
    const s = JSON.parse(localStorage.getItem('xroads-settings') ?? '{}');
    return s['anthropicApiKey'] ?? '';
  } catch { return ''; }
}

export function ChatPanel() {
  const { session, projectPath } = useAppStore();
  const [messages, setMessages] = useState<ChatMessage[]>([
    { id: '0', role: 'system', content: 'XRoads ready. CLI mode = real Claude conversation via terminal. API mode = direct Anthropic API with streaming.', timestamp: new Date().toISOString() },
  ]);
  const [input, setInput] = useState('');
  const [mode, setMode] = useState<ChatMode>('cli');
  const [isLoading, setIsLoading] = useState(false);
  const [abortController, setAbortController] = useState<AbortController | null>(null);
  const [detectedPRD, setDetectedPRD] = useState<{ featureName: string; description: string; stories: Array<{ id: string; title: string; priority: string }>; rawJson: string } | null>(null);
  const [showSlotAssignment, setShowSlotAssignment] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const sendMessage = useCallback(async () => {
    if (!input.trim() || isLoading) return;
    const content = input.trim();
    setInput('');

    const userMsg: ChatMessage = { id: Date.now().toString(), role: 'user', content, timestamp: new Date().toISOString() };
    setMessages(prev => [...prev, userMsg]);

    // Persist user message (fire-and-forget)
    if (session?.id) saveChatMessage(session.id, 'user', content, mode).catch(() => {});

    if (mode === 'cli') {
      await sendCliMessage(content);
    } else {
      await sendApiMessage(content);
    }
  }, [input, isLoading, mode, messages]);

  // ── CLI Mode: Real Claude conversation via `claude` CLI binary ──

  const sendCliMessage = async (content: string) => {
    setIsLoading(true);
    const placeholderId = (Date.now() + 1).toString();
    setMessages(prev => [...prev, { id: placeholderId, role: 'assistant', content: '', timestamp: new Date().toISOString(), isStreaming: true }]);

    try {
      const history = messages
        .filter(m => m.role === 'user' || m.role === 'assistant')
        .slice(-10)
        .map(m => `${m.role === 'user' ? 'Human' : 'Assistant'}: ${m.content}`)
        .join('\n\n');

      const fullPrompt = history ? `${history}\n\nHuman: ${content}` : content;

      // Call Claude CLI via Tauri backend command
      const response = await invoke<string>('run_claude_cli', {
        prompt: fullPrompt,
        systemPrompt: SYSTEM_PROMPT,
        workingDirectory: projectPath ?? undefined,
      }).catch(() => null);

      if (response) {
        setMessages(prev => prev.map(m => m.id === placeholderId ? { ...m, content: response, isStreaming: false } : m));
        // Persist assistant response
        if (session?.id) saveChatMessage(session.id, 'assistant', response, 'cli').catch(() => {});
        // Detect PRD in response
        const prd = detectPRDInResponse(response);
        if (prd) setDetectedPRD(prd);
      } else {
        setMessages(prev => prev.map(m => m.id === placeholderId
          ? { ...m, role: 'error' as MessageRole, content: 'Claude CLI not found. Install `claude` (Claude Code) or switch to API mode.\n\nInstall: npm install -g @anthropic-ai/claude-code', isStreaming: false }
          : m
        ));
      }
    } catch (err) {
      setMessages(prev => prev.map(m => m.id === placeholderId
        ? { ...m, role: 'error' as MessageRole, content: `CLI error: ${err}`, isStreaming: false }
        : m
      ));
    } finally {
      setIsLoading(false);
    }
  };

  // ── API Mode: Direct Anthropic API with SSE streaming ──

  const sendApiMessage = async (content: string) => {
    const apiKey = loadApiKey();
    if (!apiKey) {
      setMessages(prev => [...prev, { id: (Date.now() + 1).toString(), role: 'error', content: 'No API key. Go to Settings → API Keys → Anthropic.', timestamp: new Date().toISOString() }]);
      return;
    }

    setIsLoading(true);
    const controller = new AbortController();
    setAbortController(controller);
    const placeholderId = (Date.now() + 1).toString();
    setMessages(prev => [...prev, { id: placeholderId, role: 'assistant', content: '', timestamp: new Date().toISOString(), isStreaming: true }]);

    try {
      const apiMessages = messages
        .filter(m => m.role === 'user' || m.role === 'assistant')
        .slice(-20)
        .map(m => ({ role: m.role, content: m.content }));
      apiMessages.push({ role: 'user', content });

      const res = await fetch('https://api.anthropic.com/v1/messages', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-api-key': apiKey,
          'anthropic-version': '2023-06-01',
          'anthropic-dangerous-direct-browser-access': 'true',
        },
        body: JSON.stringify({ model: 'claude-sonnet-4-20250514', max_tokens: 4096, system: SYSTEM_PROMPT, messages: apiMessages, stream: true }),
        signal: controller.signal,
      });

      if (!res.ok) throw new Error(res.status === 401 ? 'Invalid API key' : res.status === 429 ? 'Rate limited' : `API ${res.status}`);

      const reader = res.body?.getReader();
      const decoder = new TextDecoder();
      let acc = '';
      let buf = '';

      while (reader) {
        const { done, value } = await reader.read();
        if (done) break;
        buf += decoder.decode(value, { stream: true });
        const lines = buf.split('\n');
        buf = lines.pop() ?? '';
        for (const line of lines) {
          if (!line.startsWith('data: ') || line === 'data: [DONE]') continue;
          try {
            const d = JSON.parse(line.slice(6));
            if (d.type === 'content_block_delta' && d.delta?.text) {
              acc += d.delta.text;
              setMessages(prev => prev.map(m => m.id === placeholderId ? { ...m, content: acc } : m));
            }
          } catch { /* skip */ }
        }
      }

      setMessages(prev => prev.map(m => m.id === placeholderId ? { ...m, isStreaming: false, content: acc || '(empty)' } : m));
      // Persist assistant response
      if (session?.id && acc) saveChatMessage(session.id, 'assistant', acc, 'api').catch(() => {});
      // Detect PRD in response
      const prd = detectPRDInResponse(acc);
      if (prd) setDetectedPRD(prd);
    } catch (err: any) {
      if (err.name === 'AbortError') {
        setMessages(prev => prev.map(m => m.id === placeholderId ? { ...m, content: m.content + '\n*(cancelled)*', isStreaming: false } : m));
      } else {
        setMessages(prev => prev.map(m => m.id === placeholderId ? { ...m, role: 'error' as MessageRole, content: `${err.message ?? err}`, isStreaming: false } : m));
      }
    } finally {
      setIsLoading(false);
      setAbortController(null);
    }
  };

  const suggestions = mode === 'cli' ? CLI_SUGGESTIONS : API_SUGGESTIONS;

  return (
    <div className="w-72 flex flex-col border-r border-gray-800 bg-gray-950">
      <div className="px-3 py-2 border-b border-gray-800 flex items-center justify-between">
        <span className="text-[11px] font-bold font-mono text-gray-300 tracking-wider">ORCHESTRATOR</span>
        <div className="flex gap-1">
          {(['cli', 'api'] as const).map((m) => (
            <button key={m} onClick={() => setMode(m)}
              className={`px-1.5 py-0.5 text-[9px] font-mono rounded transition-colors ${mode === m ? 'bg-neon-green/20 text-neon-green' : 'text-gray-500 hover:text-gray-400'}`}>
              {m.toUpperCase()}
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-auto p-3 space-y-3">
        {messages.map((msg) => (
          <div key={msg.id} className="space-y-0.5 xr-animate-fade-in">
            <div className="flex items-center gap-1.5">
              <span className={`text-[9px] font-bold font-mono ${msg.role === 'user' ? 'text-neon-green' : msg.role === 'error' ? 'text-red-400' : msg.role === 'system' ? 'text-gray-500' : 'text-neon-blue'}`}>
                {msg.role === 'user' ? 'YOU' : msg.role === 'system' ? 'SYS' : msg.role === 'error' ? 'ERR' : 'CLAUDE'}
              </span>
              <span className="text-[8px] text-gray-600 font-mono">
                {new Date(msg.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
              </span>
              {msg.isStreaming && <span className="text-[8px] text-neon-green animate-pulse">streaming...</span>}
            </div>
            <div className={`text-[11px] font-mono leading-relaxed whitespace-pre-wrap break-words ${msg.role === 'user' ? 'text-neon-green' : msg.role === 'error' ? 'text-red-400' : msg.role === 'system' ? 'text-gray-500 italic' : 'text-gray-300'}`}>
              {msg.content || (msg.isStreaming ? '...' : '')}
            </div>
          </div>
        ))}
        <div ref={bottomRef} />
      </div>

      {/* PRD Detected Overlay */}
      {detectedPRD && (
        <PRDDetectedOverlay
          prd={detectedPRD}
          onLaunchSingle={(_agentType, _branch) => {
            setDetectedPRD(null);
          }}
          onConfigureMultiAgent={() => setShowSlotAssignment(true)}
          onViewPRD={() => {
            setMessages(prev => [...prev, {
              id: Date.now().toString(), role: 'system',
              content: '```json\n' + detectedPRD.rawJson + '\n```',
              timestamp: new Date().toISOString(),
            }]);
          }}
          onDismiss={() => setDetectedPRD(null)}
        />
      )}

      {/* Slot Assignment Dialog (full-screen) */}
      {showSlotAssignment && detectedPRD && (
        <SlotAssignmentDialog
          featureName={detectedPRD.featureName}
          stories={detectedPRD.stories}
          prdJson={detectedPRD.rawJson}
          projectPath={projectPath ?? ''}
          onClose={() => setShowSlotAssignment(false)}
          onLaunched={() => { setShowSlotAssignment(false); setDetectedPRD(null); }}
        />
      )}

      <div className="px-3 py-1.5 flex flex-wrap gap-1 border-t border-gray-800/50">
        {suggestions.map((s) => (
          <button key={s} onClick={() => setInput(s)}
            className="px-2 py-0.5 text-[9px] font-mono text-gray-500 bg-gray-800/50 rounded hover:text-neon-green hover:bg-gray-800 transition-colors truncate max-w-[130px]">
            {s}
          </button>
        ))}
      </div>

      <div className="p-2 border-t border-gray-800">
        <div className="flex gap-1.5">
          <input type="text" value={input} onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && sendMessage()} disabled={isLoading}
            placeholder={mode === 'cli' ? 'Talk to Claude...' : 'Ask anything...'}
            className="flex-1 bg-gray-900 border border-gray-700 rounded px-2 py-1.5 text-[11px] font-mono text-gray-200 placeholder:text-gray-600 focus:border-neon-green/40 focus:outline-none disabled:opacity-50" />
          {isLoading ? (
            <button onClick={() => abortController?.abort()}
              className="px-2.5 py-1.5 bg-red-500/20 text-red-400 text-[10px] font-mono rounded border border-red-500/30 hover:bg-red-500/30 transition-colors">■</button>
          ) : (
            <button onClick={sendMessage}
              className="px-2.5 py-1.5 bg-neon-green/20 text-neon-green text-[10px] font-mono rounded border border-neon-green/30 hover:bg-neon-green/30 transition-colors">↵</button>
          )}
        </div>
      </div>

      <div className="px-3 py-1 border-t border-gray-800 flex items-center gap-2 text-[8px] font-mono text-gray-600">
        <span className={`w-1.5 h-1.5 rounded-full ${isLoading ? 'bg-amber-400 animate-pulse' : 'bg-emerald-500'}`} />
        <span className="truncate">{projectPath?.split('/').pop() ?? 'No project'}</span>
        <span>|</span>
        <span>{session?.status ?? 'idle'}</span>
        <span>|</span>
        <span className={mode === 'api' ? 'text-neon-blue' : 'text-neon-green'}>{mode}</span>
      </div>
    </div>
  );
}
