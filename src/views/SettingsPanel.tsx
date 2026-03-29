import { useState, useEffect } from 'react';
import * as api from '../services/api';

interface CliTool {
  name: string;
  available: boolean;
  version?: string;
  path?: string;
}

export function SettingsPanel({ onClose }: { onClose: () => void }) {
  const [tab, setTab] = useState<'general' | 'cli' | 'api'>('general');
  const [tools, setTools] = useState<CliTool[]>([]);
  const [repoPath, setRepoPath] = useState('');
  const [apiKeys, setApiKeys] = useState({ anthropic: '', openai: '', google: '' });

  useEffect(() => {
    api.detectCliTools().then(setTools).catch(() => {});
  }, []);

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-gray-900 rounded-xl border border-gray-700 w-[600px] max-h-[500px] flex flex-col" onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div className="flex items-center gap-4 px-4 py-3 border-b border-gray-800">
          <h2 className="text-sm font-bold font-mono text-gray-200">Settings</h2>
          <div className="flex-1" />
          <button onClick={onClose} className="text-gray-500 hover:text-gray-300 text-sm">✕</button>
        </div>

        {/* Tabs */}
        <div className="flex border-b border-gray-800">
          {(['general', 'cli', 'api'] as const).map((t) => (
            <button key={t} onClick={() => setTab(t)}
              className={`px-4 py-2 text-[11px] font-mono ${tab === t ? 'text-neon-green border-b-2 border-neon-green' : 'text-gray-500 hover:text-gray-300'}`}>
              {t.toUpperCase()}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="flex-1 overflow-auto p-4">
          {tab === 'general' && (
            <div className="space-y-4">
              <label className="block">
                <span className="text-[10px] font-mono text-gray-400 block mb-1">Default Repository Path</span>
                <input value={repoPath} onChange={(e) => setRepoPath(e.target.value)}
                  placeholder="/path/to/your/repo"
                  className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-1.5 text-[11px] font-mono text-gray-200 focus:outline-none focus:border-neon-green/50" />
              </label>
            </div>
          )}

          {tab === 'cli' && (
            <div className="space-y-3">
              <h3 className="text-[10px] font-bold font-mono text-gray-500">CLI TOOLS</h3>
              {tools.map((tool) => (
                <div key={tool.name} className="flex items-center gap-3 p-2 bg-gray-800/50 rounded-lg">
                  <span className={`w-2 h-2 rounded-full ${tool.available ? 'bg-emerald-400' : 'bg-red-400'}`} />
                  <span className="text-[11px] font-mono text-gray-200 w-20">{tool.name}</span>
                  <span className="text-[10px] font-mono text-gray-500 flex-1 truncate">{tool.version ?? 'not found'}</span>
                  <span className="text-[9px] font-mono text-gray-600 truncate max-w-40">{tool.path ?? ''}</span>
                </div>
              ))}
            </div>
          )}

          {tab === 'api' && (
            <div className="space-y-4">
              {Object.entries(apiKeys).map(([key, value]) => (
                <label key={key} className="block">
                  <span className="text-[10px] font-mono text-gray-400 block mb-1">{key.toUpperCase()} API Key</span>
                  <input value={value} onChange={(e) => setApiKeys((prev) => ({ ...prev, [key]: e.target.value }))}
                    type="password" placeholder={`sk-...`}
                    className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-1.5 text-[11px] font-mono text-gray-200 focus:outline-none focus:border-neon-green/50" />
                </label>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
