import { useState, useEffect } from 'react';
import { useAppStore } from '../stores/appStore';
import { getLogBuffer, onLogBufferChange } from '../services/eventBus';
import * as api from '../services/api';
import type { LogEntryPayload } from '../models';

type Tab = 'project' | 'mcp';

interface GitInfo {
  branch: string;
  commits: Array<{ shortSha: string; message: string; author: string; date: string }>;
  branches: string[];
}

const LOG_LEVEL_COLORS: Record<string, string> = {
  debug: 'text-gray-500',
  info: 'text-neon-blue',
  warn: 'text-amber-400',
  error: 'text-red-400',
};

export function GitPanel() {
  const { projectPath } = useAppStore();
  const [info, setInfo] = useState<GitInfo | null>(null);
  const [tab, setTab] = useState<Tab>('project');
  const [logs, setLogs] = useState<LogEntryPayload[]>(() => getLogBuffer().slice(-50));

  useEffect(() => {
    if (!projectPath) return;
    loadGitInfo(projectPath);
  }, [projectPath]);

  useEffect(() => {
    const unsub = onLogBufferChange((entries) => {
      setLogs(entries.slice(-50));
    });
    return unsub;
  }, []);

  const loadGitInfo = async (path: string) => {
    try {
      const [branch, commits, branches] = await Promise.all([
        api.gitCurrentBranch(path),
        api.gitRecentCommits(path, 10),
        api.gitBranches(path),
      ]);
      setInfo({ branch, commits, branches });
    } catch {
      setInfo(null);
    }
  };

  return (
    <div className="w-72 flex flex-col border-l border-gray-800 bg-gray-950">
      {/* Tab header */}
      <div className="flex border-b border-gray-800">
        {(['project', 'mcp'] as const).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={`flex-1 px-3 py-2 text-[10px] font-bold font-mono uppercase tracking-wider transition-colors ${
              tab === t ? 'text-neon-green border-b-2 border-neon-green' : 'text-gray-500 hover:text-gray-400'
            }`}
          >
            {t === 'project' ? 'PROJECT' : 'MCP LOGS'}
          </button>
        ))}
      </div>

      {tab === 'project' ? (
        /* PROJECT tab */
        !info ? (
          <div className="flex-1 flex items-center justify-center">
            <p className="text-[10px] text-gray-600 font-mono">No git repo loaded</p>
          </div>
        ) : (
          <div className="flex-1 overflow-auto">
            {/* Status header */}
            <div className="p-3 border-b border-gray-800/50 flex items-center gap-2">
              <span className="w-2 h-2 rounded-full bg-emerald-500" />
              <span className="text-[10px] font-mono text-gray-300">⎇ {info.branch}</span>
              <span className="text-[9px] font-mono text-gray-600">{info.commits.length} commits</span>
            </div>

            {/* Recent commits */}
            <div className="p-3">
              <h3 className="text-[9px] font-bold font-mono text-gray-500 mb-2">RECENT</h3>
              <div className="space-y-1.5">
                {info.commits.map((c) => (
                  <div key={c.shortSha} className="flex items-start gap-2 text-[10px] font-mono">
                    <span className="text-neon-blue shrink-0">{c.shortSha}</span>
                    <span className="text-gray-300 truncate flex-1">{c.message}</span>
                    <span className="text-gray-600 shrink-0">{timeAgo(c.date)}</span>
                  </div>
                ))}
              </div>
            </div>

            {/* Branches */}
            <div className="p-3 border-t border-gray-800">
              <h3 className="text-[9px] font-bold font-mono text-gray-500 mb-2">BRANCHES ({info.branches.length})</h3>
              <div className="space-y-1 max-h-32 overflow-auto">
                {info.branches.map((b) => (
                  <div key={b} className={`text-[10px] font-mono ${b === info.branch ? 'text-neon-green' : 'text-gray-400'}`}>
                    {b === info.branch ? '● ' : '  '}{b}
                  </div>
                ))}
              </div>
            </div>

            {/* GitMaster */}
            <div className="p-3 border-t border-gray-800">
              <div className="flex items-center gap-2 mb-2">
                <h3 className="text-[9px] font-bold font-mono text-gray-500">GIT MASTER</h3>
                <span className="text-[8px] font-mono text-emerald-400 bg-emerald-400/10 px-1 py-0.5 rounded">Ready</span>
              </div>
              <div className="flex items-center gap-2 p-2 bg-gray-900/50 rounded text-[10px] font-mono text-gray-500">
                <span>🔀</span>
                <span>idle</span>
                <span className="text-gray-700">target: {info.branch}</span>
              </div>
            </div>
          </div>
        )
      ) : (
        /* MCP LOGS tab */
        <div className="flex-1 overflow-auto p-2 space-y-1">
          {logs.length === 0 ? (
            <div className="text-[10px] text-gray-600 font-mono text-center mt-8">No MCP logs yet</div>
          ) : (
            logs.map((log, i) => (
              <div key={i} className="flex items-start gap-1.5 text-[9px] font-mono">
                <span className={`shrink-0 font-bold uppercase ${LOG_LEVEL_COLORS[log.level] ?? 'text-gray-500'}`}>
                  {log.level.slice(0, 3)}
                </span>
                <span className="text-gray-600 shrink-0">
                  {new Date(log.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
                </span>
                <span className="text-gray-400 truncate">{log.message}</span>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}

function timeAgo(dateStr: string): string {
  const diff = Date.now() - new Date(dateStr).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 60) return `${mins}m`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h`;
  return `${Math.floor(hours / 24)}d`;
}
