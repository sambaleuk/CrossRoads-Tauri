import { useState, useRef, useEffect, useCallback, type ReactNode } from 'react';
import { useAppStore } from '../stores/appStore';
import {
  allAgentHealth,
  costSummarySession,
  cockpitPause,
  cockpitResume,
  cockpitClose,
  fetchTrustScores,
  fetchPerformanceProfiles,
} from '../services/api';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type MessageRole = 'user' | 'assistant' | 'system' | 'error';
type ApiMode = 'cli' | 'api';

interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  timestamp: string;
  isStreaming?: boolean;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CLI_SUGGESTIONS = ['show status', 'show costs', 'pause all', 'help'];
const API_SUGGESTIONS = ['Write a PRD for...', 'Explain this code', 'Analyze dependencies'];

const ANTHROPIC_API_URL = 'https://api.anthropic.com/v1/messages';
const ANTHROPIC_MODEL = 'claude-sonnet-4-20250514';
const SYSTEM_PROMPT =
  'You are the XRoads orchestrator assistant. Help the user manage their multi-agent coding sessions. You can explain features, suggest PRDs, analyze code patterns. Be concise and direct.';

const roleStyles: Record<MessageRole, string> = {
  user: 'text-neon-green',
  assistant: 'text-gray-300',
  system: 'text-gray-500 italic',
  error: 'text-red-400',
};

const roleLabelsCli: Record<MessageRole, string> = {
  user: 'YOU',
  assistant: 'ORCH',
  system: 'SYS',
  error: 'ERR',
};

const roleLabelsApi: Record<MessageRole, string> = {
  user: 'YOU',
  assistant: 'AI',
  system: 'SYS',
  error: 'ERR',
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function uid(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

function formatTime(iso: string): string {
  return new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

/** Render markdown-ish text: **bold**, `inline code`, ```code blocks``` */
function renderFormattedContent(text: string): ReactNode {
  const parts: ReactNode[] = [];
  // Split on code blocks first
  const codeBlockRegex = /```([\s\S]*?)```/g;
  let lastIdx = 0;
  let match: RegExpExecArray | null;

  while ((match = codeBlockRegex.exec(text)) !== null) {
    if (match.index > lastIdx) {
      parts.push(<span key={lastIdx}>{renderInline(text.slice(lastIdx, match.index))}</span>);
    }
    parts.push(
      <pre key={match.index} className="bg-gray-900 border border-gray-700 rounded px-2 py-1 my-1 overflow-x-auto text-[10px] text-gray-300 whitespace-pre-wrap">
        {match[1].trim()}
      </pre>
    );
    lastIdx = match.index + match[0].length;
  }
  if (lastIdx < text.length) {
    parts.push(<span key={lastIdx}>{renderInline(text.slice(lastIdx))}</span>);
  }
  return <>{parts}</>;
}

function renderInline(text: string): ReactNode {
  const parts: ReactNode[] = [];
  const inlineRegex = /(\*\*(.+?)\*\*|`([^`]+)`)/g;
  let lastIdx = 0;
  let match: RegExpExecArray | null;

  while ((match = inlineRegex.exec(text)) !== null) {
    if (match.index > lastIdx) {
      parts.push(text.slice(lastIdx, match.index));
    }
    if (match[2]) {
      parts.push(<strong key={match.index} className="font-bold text-white">{match[2]}</strong>);
    } else if (match[3]) {
      parts.push(
        <code key={match.index} className="bg-gray-800 px-1 rounded text-neon-green text-[10px]">{match[3]}</code>
      );
    }
    lastIdx = match.index + match[0].length;
  }
  if (lastIdx < text.length) {
    parts.push(text.slice(lastIdx));
  }
  return <>{parts}</>;
}

// ---------------------------------------------------------------------------
// CLI Command Dispatcher
// ---------------------------------------------------------------------------

async function dispatchCliCommand(
  raw: string,
  sessionId: string | null,
  slots: ReturnType<typeof useAppStore.getState>['slots'],
  sessionStatus: string | undefined,
): Promise<string> {
  const input = raw.trim().toLowerCase();

  // help
  if (input === 'help') {
    return [
      '--- Available Commands ---',
      'start orchestration  Start orchestration (requires active session + PRD)',
      'load prd             Load a PRD file (placeholder)',
      'show status / status Show session & slot statuses',
      'show agents          Show all agent health',
      'show costs / costs   Show session cost summary',
      'pause / pause all    Pause the session',
      'resume               Resume the session',
      'stop / stop all      Close the session',
      'show trust           Show trust scores',
      'show profiles        Show performance profiles',
      'help                 Show this message',
    ].join('\n');
  }

  // status
  if (input === 'show status' || input === 'status') {
    const lines = [`Session: ${sessionId ?? 'none'}`, `Status:  ${sessionStatus ?? 'idle'}`];
    if (slots.length === 0) {
      lines.push('Slots:   (none)');
    } else {
      lines.push(`Slots:   ${slots.length}`);
      for (const s of slots) {
        lines.push(`  [${s.slotIndex}] ${s.agentType.padEnd(7)} ${s.status.padEnd(16)} ${s.currentTask ?? ''}`);
      }
    }
    return lines.join('\n');
  }

  // show agents
  if (input === 'show agents' || input === 'agent status') {
    const healths = await allAgentHealth();
    if (healths.length === 0) return 'No agents registered.';
    return healths
      .map((h) => `[${h.slotId.slice(0, 8)}] ${h.agentType.padEnd(7)} status=${h.status} progress=${h.progressPct ?? '-'}% task=${h.currentTask ?? '-'}`)
      .join('\n');
  }

  // costs
  if (input === 'show costs' || input === 'costs') {
    if (!sessionId) return 'No active session.';
    const summary = await costSummarySession(sessionId);
    return [
      '--- Cost Summary ---',
      `Input tokens:  ${summary.totalInputTokens.toLocaleString()}`,
      `Output tokens: ${summary.totalOutputTokens.toLocaleString()}`,
      `Total cost:    $${(summary.totalCostCents / 100).toFixed(4)}`,
      `Events:        ${summary.eventCount}`,
    ].join('\n');
  }

  // pause
  if (input === 'pause' || input === 'pause all') {
    if (!sessionId) return 'No active session to pause.';
    await cockpitPause(sessionId);
    return 'Session paused.';
  }

  // resume
  if (input === 'resume') {
    if (!sessionId) return 'No active session to resume.';
    await cockpitResume(sessionId);
    return 'Session resumed.';
  }

  // stop
  if (input === 'stop' || input === 'stop all') {
    if (!sessionId) return 'No active session to stop.';
    await cockpitClose(sessionId);
    return 'Session closed.';
  }

  // trust
  if (input === 'show trust' || input === 'trust scores') {
    const scores = await fetchTrustScores();
    if (scores.length === 0) return 'No trust scores recorded yet.';
    return scores
      .map((t) => `${t.agentType.padEnd(8)} ${t.domain.padEnd(16)} score=${t.score.toFixed(2)} stories=${t.totalStories} (${t.successfulStories} ok)`)
      .join('\n');
  }

  // profiles
  if (input === 'show profiles' || input === 'performance') {
    const profiles = await fetchPerformanceProfiles();
    if (profiles.length === 0) return 'No performance profiles yet.';
    return profiles
      .map((p) => `${p.agentType.padEnd(8)} ${p.taskCategory.padEnd(16)} avg=${p.avgDurationMs}ms success=${(p.successRate * 100).toFixed(0)}%`)
      .join('\n');
  }

  // start orchestration
  if (input === 'start orchestration') {
    if (!sessionId) return 'No active session. Create a session first.';
    return 'Orchestration requires a PRD path. Use: load prd';
  }

  // load prd
  if (input === 'load prd') {
    return 'PRD file picker not yet integrated. Provide a PRD path directly via start orchestration.';
  }

  return "Unknown command. Type 'help' for available commands.";
}

// ---------------------------------------------------------------------------
// Anthropic Streaming API
// ---------------------------------------------------------------------------

async function streamAnthropicResponse(
  messages: Array<{ role: string; content: string }>,
  apiKey: string,
  onDelta: (text: string) => void,
  signal: AbortSignal,
): Promise<void> {
  const response = await fetch(ANTHROPIC_API_URL, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'x-api-key': apiKey,
      'anthropic-version': '2023-06-01',
      'anthropic-dangerous-direct-browser-access': 'true',
    },
    body: JSON.stringify({
      model: ANTHROPIC_MODEL,
      max_tokens: 2048,
      system: SYSTEM_PROMPT,
      stream: true,
      messages,
    }),
    signal,
  });

  if (!response.ok) {
    const body = await response.text();
    if (response.status === 401) throw new Error('Invalid API key. Check Settings > API Keys.');
    if (response.status === 429) throw new Error('Rate limited. Please wait and try again.');
    throw new Error(`API error ${response.status}: ${body.slice(0, 200)}`);
  }

  const reader = response.body?.getReader();
  if (!reader) throw new Error('No response body');

  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split('\n');
    buffer = lines.pop() ?? '';

    for (const line of lines) {
      if (line.startsWith('data: ')) {
        const data = line.slice(6).trim();
        if (data === '[DONE]') return;
        try {
          const event = JSON.parse(data);
          if (event.type === 'content_block_delta' && event.delta?.text) {
            onDelta(event.delta.text);
          }
        } catch {
          // skip non-JSON lines
        }
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function ChatPanel() {
  const { session, projectPath, slots } = useAppStore();
  const [messages, setMessages] = useState<ChatMessage[]>([
    {
      id: uid(),
      role: 'system',
      content: 'XRoads orchestrator ready. Type "help" for commands or switch to API mode for AI chat.',
      timestamp: new Date().toISOString(),
    },
  ]);
  const [input, setInput] = useState('');
  const [apiMode, setApiMode] = useState<ApiMode>('cli');
  const [isLoading, setIsLoading] = useState(false);
  const abortRef = useRef<AbortController | null>(null);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Add a message to state
  const addMessage = useCallback((role: MessageRole, content: string, isStreaming = false): string => {
    const id = uid();
    setMessages((prev) => [...prev, { id, role, content, timestamp: new Date().toISOString(), isStreaming }]);
    return id;
  }, []);

  // Update a message by id
  const updateMessage = useCallback((id: string, updater: (msg: ChatMessage) => ChatMessage) => {
    setMessages((prev) => prev.map((m) => (m.id === id ? updater(m) : m)));
  }, []);

  // Handle CLI command
  const handleCliCommand = useCallback(
    async (text: string) => {
      setIsLoading(true);
      try {
        const result = await dispatchCliCommand(text, session?.id ?? null, slots, session?.status);
        addMessage('assistant', result);
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        addMessage('error', `Command failed: ${msg}`);
      } finally {
        setIsLoading(false);
      }
    },
    [session, slots, addMessage],
  );

  // Handle API chat
  const handleApiChat = useCallback(
    async (text: string) => {
      const apiKey = localStorage.getItem('anthropicApiKey');
      if (!apiKey) {
        addMessage('error', 'No API key found. Set your Anthropic API key in Settings > API Keys.');
        return;
      }

      setIsLoading(true);
      const controller = new AbortController();
      abortRef.current = controller;

      // Build conversation history (last 20 messages)
      const history = messages
        .filter((m) => m.role === 'user' || m.role === 'assistant')
        .slice(-20)
        .map((m) => ({ role: m.role, content: m.content }));
      history.push({ role: 'user', content: text });

      const assistantId = addMessage('assistant', '', true);

      try {
        let accumulated = '';
        await streamAnthropicResponse(
          history,
          apiKey,
          (delta) => {
            accumulated += delta;
            const current = accumulated;
            updateMessage(assistantId, (m) => ({ ...m, content: current }));
          },
          controller.signal,
        );

        // Finalize
        updateMessage(assistantId, (m) => ({ ...m, isStreaming: false }));

        // Check for PRD JSON in response
        if (accumulated.includes('"stories"') && accumulated.includes('"userStory"')) {
          addMessage('system', 'Detected PRD structure in response. Copy it to a .json file to use with orchestration.');
        }
      } catch (err: unknown) {
        if ((err as Error).name === 'AbortError') {
          updateMessage(assistantId, (m) => ({ ...m, content: m.content + '\n[Cancelled]', isStreaming: false }));
        } else {
          const msg = err instanceof Error ? err.message : String(err);
          updateMessage(assistantId, (m) => ({ ...m, content: '', isStreaming: false }));
          addMessage('error', msg);
        }
      } finally {
        setIsLoading(false);
        abortRef.current = null;
      }
    },
    [messages, addMessage, updateMessage],
  );

  // Send message
  const sendMessage = useCallback(() => {
    const text = input.trim();
    if (!text || isLoading) return;

    addMessage('user', text);
    setInput('');

    if (apiMode === 'cli') {
      handleCliCommand(text);
    } else {
      handleApiChat(text);
    }
  }, [input, isLoading, apiMode, addMessage, handleCliCommand, handleApiChat]);

  // Cancel streaming
  const cancelStreaming = useCallback(() => {
    abortRef.current?.abort();
  }, []);

  const suggestions = apiMode === 'cli' ? CLI_SUGGESTIONS : API_SUGGESTIONS;
  const roleLabels = apiMode === 'cli' ? roleLabelsCli : roleLabelsApi;

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
                {formatTime(msg.timestamp)}
              </span>
              {msg.isStreaming && (
                <span className="text-[8px] text-neon-green animate-pulse font-mono">streaming...</span>
              )}
            </div>
            <div
              className={`text-[11px] leading-relaxed ${roleStyles[msg.role]} ${
                msg.role === 'assistant' && apiMode === 'cli' ? 'font-mono whitespace-pre-wrap' : 'font-mono'
              }`}
            >
              {msg.role === 'assistant' && apiMode === 'api'
                ? renderFormattedContent(msg.content)
                : msg.content}
            </div>
          </div>
        ))}
        {isLoading && apiMode === 'cli' && (
          <div className="flex items-center gap-2 text-[10px] text-gray-500 font-mono">
            <span className="inline-block w-3 h-3 border-2 border-neon-green/30 border-t-neon-green rounded-full animate-spin" />
            Processing...
          </div>
        )}
        <div ref={bottomRef} />
      </div>

      {/* Suggestion chips */}
      <div className="px-3 py-1.5 flex flex-wrap gap-1 border-t border-gray-800/50">
        {suggestions.map((s) => (
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
            placeholder={apiMode === 'cli' ? 'Type a command...' : 'Ask the AI...'}
            className="flex-1 bg-gray-900 border border-gray-700 rounded px-2 py-1.5 text-[11px] font-mono text-gray-200 placeholder:text-gray-600 focus:border-neon-green/40 focus:outline-none"
            disabled={isLoading}
          />
          {isLoading && apiMode === 'api' ? (
            <button
              onClick={cancelStreaming}
              className="px-2.5 py-1.5 bg-red-500/20 text-red-400 text-[10px] font-mono rounded border border-red-500/30 hover:bg-red-500/30 transition-colors"
              title="Cancel"
            >
              ■
            </button>
          ) : (
            <button
              onClick={sendMessage}
              disabled={isLoading}
              className="px-2.5 py-1.5 bg-neon-green/20 text-neon-green text-[10px] font-mono rounded border border-neon-green/30 hover:bg-neon-green/30 transition-colors disabled:opacity-40"
            >
              ↵
            </button>
          )}
        </div>
      </div>

      {/* Status bar */}
      <div className="px-3 py-1 border-t border-gray-800 flex items-center gap-2 text-[8px] font-mono text-gray-600">
        <span
          className={`w-1.5 h-1.5 rounded-full ${
            session?.status === 'active' ? 'bg-emerald-500' : session?.status === 'paused' ? 'bg-yellow-500' : 'bg-gray-600'
          }`}
        />
        <span className="truncate">{projectPath?.split('/').pop() ?? 'No project'}</span>
        <span>|</span>
        <span>{session?.status ?? 'idle'}</span>
        <span>|</span>
        <span className={apiMode === 'api' ? 'text-blue-400' : 'text-neon-green'}>{apiMode.toUpperCase()}</span>
      </div>
    </div>
  );
}
