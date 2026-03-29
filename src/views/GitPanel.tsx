import { useState, useEffect } from 'react';
import { useAppStore } from '../stores/appStore';
import * as api from '../services/api';

interface GitInfo {
  branch: string;
  commits: Array<{ shortSha: string; message: string; author: string; date: string }>;
  branches: string[];
}

export function GitPanel() {
  const { projectPath } = useAppStore();
  const [info, setInfo] = useState<GitInfo | null>(null);

  useEffect(() => {
    if (!projectPath) return;
    loadGitInfo(projectPath);
  }, [projectPath]);

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
    <div className="flex flex-col h-full">
      <div className="p-3 border-b border-gray-800 flex items-center gap-2">
        <h2 className="text-xs font-bold font-mono text-gray-400">PROJECT</h2>
        {info && (
          <span className="text-[9px] font-mono text-emerald-400 bg-emerald-400/10 px-1.5 py-0.5 rounded">
            ⎇ {info.branch}
          </span>
        )}
      </div>

      {!info ? (
        <div className="flex-1 flex items-center justify-center">
          <p className="text-[10px] text-gray-600 font-mono">No git repo loaded</p>
        </div>
      ) : (
        <div className="flex-1 overflow-auto">
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
            <h3 className="text-[9px] font-bold font-mono text-gray-500 mb-2">BRANCHES</h3>
            <div className="space-y-1">
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
            <div className="flex items-center gap-2 p-2 bg-gray-900/50 rounded-lg text-[10px] font-mono text-gray-500">
              <span>🔀</span>
              <span>idle</span>
              <span className="text-gray-700">target: {info.branch}</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function timeAgo(dateStr: string): string {
  const diff = Date.now() - new Date(dateStr).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}
