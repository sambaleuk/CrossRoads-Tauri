import { useState } from 'react';
import type { AgentType } from '../models';

interface DetectedPRD {
  featureName: string;
  description: string;
  stories: Array<{ id: string; title: string; priority: string }>;
  rawJson: string;
}

interface Props {
  prd: DetectedPRD;
  onLaunchSingle: (agentType: AgentType, branch: string) => void;
  onConfigureMultiAgent: () => void;
  onViewPRD: () => void;
  onDismiss: () => void;
}

/**
 * PRD Detected overlay — appears in the chat when Claude generates a PRD.
 * Matches Swift's PRDProposalView behavior.
 */
export function PRDDetectedOverlay({ prd, onLaunchSingle, onConfigureMultiAgent, onViewPRD, onDismiss }: Props) {
  const [agent, setAgent] = useState<AgentType>('claude');
  const [branch, setBranch] = useState(`feat/${prd.featureName.toLowerCase().replace(/[^a-z0-9]+/g, '-')}`);

  const agents: { type: AgentType; label: string; icon: string }[] = [
    { type: 'claude', label: 'Claude Code', icon: '🧠' },
    { type: 'gemini', label: 'Gemini CLI', icon: '✨' },
    { type: 'codex', label: 'Codex', icon: '⌨️' },
  ];

  return (
    <div className="mx-2 mb-2 p-3 rounded-lg border border-neon-green/30 bg-neon-green/5 xr-animate-fade-in">
      {/* Header */}
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <span className="text-xs">📋</span>
          <span className="text-[10px] font-bold font-mono text-neon-green">PRD DETECTED</span>
        </div>
        <button onClick={onDismiss} className="text-gray-500 hover:text-gray-300 text-sm">×</button>
      </div>

      {/* PRD info */}
      <div className="mb-3">
        <div className="text-[11px] font-bold font-mono text-gray-200">{prd.featureName}</div>
        <div className="text-[9px] font-mono text-gray-500 mt-0.5 line-clamp-2">{prd.description}</div>
        <div className="text-[9px] font-mono text-gray-600 mt-1">{prd.stories.length} stories</div>
      </div>

      {/* Agent selector */}
      <div className="mb-2">
        <div className="text-[8px] font-mono text-gray-500 uppercase mb-1">Agent</div>
        <div className="flex gap-1">
          {agents.map(a => (
            <button key={a.type} onClick={() => setAgent(a.type)}
              className={`flex items-center gap-1 px-2 py-1 rounded text-[9px] font-mono transition-colors ${
                agent === a.type
                  ? 'bg-neon-green/20 text-neon-green border border-neon-green/40'
                  : 'bg-gray-800 text-gray-400 border border-gray-700 hover:border-gray-600'
              }`}>
              <span>{a.icon}</span>
              <span>{a.label}</span>
            </button>
          ))}
        </div>
      </div>

      {/* Branch */}
      <div className="mb-3">
        <div className="text-[8px] font-mono text-gray-500 uppercase mb-1">Branch</div>
        <input value={branch} onChange={e => setBranch(e.target.value)}
          className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-1 text-[10px] font-mono text-gray-300 focus:border-neon-green/40 focus:outline-none" />
      </div>

      {/* Actions */}
      <div className="flex flex-col gap-1.5">
        <div className="flex gap-1.5">
          <button onClick={onViewPRD}
            className="px-3 py-1.5 text-[9px] font-mono text-gray-400 bg-gray-800 border border-gray-700 rounded hover:bg-gray-700 transition-colors">
            View PRD
          </button>
          <button onClick={() => onLaunchSingle(agent, branch)}
            className="flex-1 px-3 py-1.5 text-[9px] font-mono text-neon-green bg-neon-green/15 border border-neon-green/30 rounded hover:bg-neon-green/25 transition-colors">
            Launch Implementation
          </button>
        </div>
        <button onClick={onConfigureMultiAgent}
          className="w-full px-3 py-1.5 text-[9px] font-mono text-neon-blue bg-neon-blue/10 border border-neon-blue/30 rounded hover:bg-neon-blue/20 transition-colors flex items-center justify-center gap-1">
          <span>👥</span>
          <span>Configure Multi-Agent →</span>
        </button>
      </div>
    </div>
  );
}

/**
 * Try to extract a PRD from Claude's response text.
 * Looks for JSON with feature_name and user_stories in code blocks or raw text.
 */
export function detectPRDInResponse(text: string): DetectedPRD | null {
  // Strategy 1: Find JSON in code blocks (handles nested braces)
  const codeBlockRegex = /```(?:json)?\s*([\s\S]*?)```/g;
  let match;

  while ((match = codeBlockRegex.exec(text)) !== null) {
    const candidate = match[1].trim();
    const parsed = tryParsePRD(candidate);
    if (parsed) return parsed;
  }

  // Strategy 2: Find the largest JSON-like block in the text
  const jsonStart = text.indexOf('{"feature_name"');
  if (jsonStart >= 0) {
    // Find matching closing brace by counting depth
    let depth = 0;
    let end = jsonStart;
    for (let i = jsonStart; i < text.length; i++) {
      if (text[i] === '{') depth++;
      if (text[i] === '}') depth--;
      if (depth === 0) { end = i + 1; break; }
    }
    const candidate = text.slice(jsonStart, end);
    const parsed = tryParsePRD(candidate);
    if (parsed) return parsed;
  }

  // Strategy 3: Try the entire text as JSON
  const parsed = tryParsePRD(text);
  if (parsed) return parsed;

  return null;
}

function tryParsePRD(text: string): DetectedPRD | null {
  try {
    const parsed = JSON.parse(text);
    if (parsed.feature_name && parsed.user_stories && Array.isArray(parsed.user_stories)) {
      return {
        featureName: parsed.feature_name,
        description: parsed.description ?? '',
        stories: parsed.user_stories.map((s: any) => ({
          id: s.id ?? '',
          title: s.title ?? '',
          priority: s.priority ?? 'medium',
        })),
        rawJson: text,
      };
    }
  } catch { /* not valid JSON */ }
  return null;
}
