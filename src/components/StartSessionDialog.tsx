import { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import * as api from '../services/api';
import { useAppStore } from '../stores/appStore';

type SessionMode = 'single' | 'multi';

interface Props {
  onClose: () => void;
  onStarted: () => void;
}

/**
 * Start Session dialog — the primary onboarding flow.
 * Matches the Swift StartSessionSheet experience:
 * 1. Project (path + browse + git status)
 * 2. Feature (name)
 * 3. Mode (Single / Multi-Agent)
 * 4. Start
 */
export function StartSessionDialog({ onClose, onStarted }: Props) {
  const { projectPath, setProjectPath } = useAppStore();

  const [path, setPath] = useState(projectPath ?? '');
  const [isGitRepo, setIsGitRepo] = useState(false);
  const [branch, setBranch] = useState('');
  const [featureName, setFeatureName] = useState('');
  const [mode, setMode] = useState<SessionMode>('multi');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  // Browse for folder
  const browse = async () => {
    try {
      const selected = await open({ directory: true, multiple: false, title: 'Select project directory' });
      if (selected && typeof selected === 'string') {
        setPath(selected);
        checkGit(selected);
      }
    } catch {}
  };

  // Check git status
  const checkGit = async (p: string) => {
    try {
      const git = await api.isGitRepo(p);
      setIsGitRepo(git);
      if (git) {
        const b = await api.gitCurrentBranch(p);
        setBranch(b);
      } else {
        setBranch('');
      }
    } catch {
      setIsGitRepo(false);
      setBranch('');
    }
  };

  // Start session
  const start = async () => {
    if (!path.trim()) { setError('Select a project directory'); return; }
    if (!featureName.trim()) { setError('Enter a feature name'); return; }

    setLoading(true);
    setError('');

    try {
      setProjectPath(path);

      // Create workspace
      const name = path.split('/').pop() ?? 'project';
      try { await api.createWorkspace(name, path); } catch { /* exists */ }

      // Create session
      const session = await api.createSession(path);
      useAppStore.getState().setSession(session);

      // Activate cockpit (if multi-agent)
      if (mode === 'multi') {
        try {
          await api.cockpitActivate(session.id);
          // Fetch slots
          const slots = await api.fetchSlots(session.id);
          useAppStore.getState().setSlots(slots);
        } catch {
          // Activation may fail if not a git repo — session still created
        }
      }

      onStarted();
    } catch (err: any) {
      setError(err?.message ?? 'Failed to start session');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" onClick={onClose}>
      <div className="w-[480px] bg-gray-900 border border-gray-700 rounded-xl shadow-2xl xr-animate-scale-in"
        onClick={e => e.stopPropagation()}>

        {/* Header */}
        <div className="px-6 pt-5 pb-3">
          <h2 className="text-base font-bold font-mono text-gray-100">Start Session</h2>
          <p className="text-[10px] font-mono text-gray-500 mt-1">Configure and launch an AI development session</p>
        </div>

        <div className="px-6 pb-5 space-y-5">

          {/* Step 1: Project */}
          <div className="space-y-2">
            <StepLabel num={1} label="Project" />
            <div className="flex gap-2">
              <input
                type="text" value={path}
                onChange={e => { setPath(e.target.value); if (e.target.value.length > 3) checkGit(e.target.value); }}
                placeholder="/path/to/your/project"
                className="flex-1 bg-gray-800 border border-gray-700 rounded px-3 py-2 text-[11px] font-mono text-gray-200 placeholder:text-gray-600 focus:border-neon-green/40 focus:outline-none"
              />
              <button onClick={browse}
                className="px-3 py-2 text-[10px] font-mono bg-gray-800 text-gray-300 border border-gray-700 rounded hover:bg-gray-700 transition-colors">
                Browse
              </button>
            </div>
            {path && (
              <div className="flex items-center gap-2 text-[9px] font-mono">
                <span className={`w-2 h-2 rounded-full ${isGitRepo ? 'bg-emerald-400' : 'bg-red-400'}`} />
                <span className={isGitRepo ? 'text-emerald-400' : 'text-red-400'}>
                  {isGitRepo ? 'Git repo' : 'Not a git repo'}
                </span>
                {branch && (
                  <>
                    <span className="text-gray-600">•</span>
                    <span className="text-gray-400">⎇ {branch}</span>
                  </>
                )}
              </div>
            )}
          </div>

          {/* Step 2: Feature */}
          <div className="space-y-2">
            <StepLabel num={2} label="Feature" />
            <input
              type="text" value={featureName}
              onChange={e => setFeatureName(e.target.value)}
              placeholder="e.g. user-authentication, dark-mode, api-refactor"
              className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-[11px] font-mono text-gray-200 placeholder:text-gray-600 focus:border-neon-green/40 focus:outline-none"
            />
          </div>

          {/* Step 3: Mode */}
          <div className="space-y-2">
            <StepLabel num={3} label="Mode" />
            <div className="grid grid-cols-2 gap-3">
              <ModeCard
                icon="👤"
                title="Single Agent"
                desc="One agent works in the main directory"
                selected={mode === 'single'}
                onClick={() => setMode('single')}
              />
              <ModeCard
                icon="👥"
                title="Multi-Agent (Parallel)"
                desc="Multiple agents work on parallel worktrees"
                selected={mode === 'multi'}
                onClick={() => setMode('multi')}
              />
            </div>
          </div>

          {/* Error */}
          {error && (
            <div className="text-[10px] font-mono text-red-400 bg-red-400/10 border border-red-400/20 rounded px-3 py-2">
              {error}
            </div>
          )}

          {/* Actions */}
          <div className="flex justify-end gap-3 pt-2">
            <button onClick={onClose}
              className="px-4 py-2 text-[11px] font-mono text-gray-400 hover:text-gray-200 transition-colors">
              Cancel
            </button>
            <button onClick={start} disabled={loading}
              className="px-5 py-2 text-[11px] font-mono bg-neon-green/20 text-neon-green border border-neon-green/30 rounded hover:bg-neon-green/30 transition-colors disabled:opacity-50">
              {loading ? 'Starting...' : 'Start Session'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function StepLabel({ num, label }: { num: number; label: string }) {
  return (
    <div className="flex items-center gap-2">
      <span className="w-5 h-5 rounded-full bg-neon-green/20 text-neon-green text-[10px] font-bold font-mono flex items-center justify-center">{num}</span>
      <span className="text-[11px] font-bold font-mono text-gray-300">{label}</span>
    </div>
  );
}

function ModeCard({ icon, title, desc, selected, onClick }: {
  icon: string; title: string; desc: string; selected: boolean; onClick: () => void;
}) {
  return (
    <button onClick={onClick}
      className={`p-4 rounded-lg border text-left transition-all ${
        selected
          ? 'border-neon-green/50 bg-neon-green/10 shadow-[0_0_12px_var(--xr-neon-green)]'
          : 'border-gray-700 bg-gray-800/40 hover:border-gray-600'
      }`}>
      <div className="text-xl mb-2">{icon}</div>
      <div className="text-[11px] font-bold font-mono text-gray-200">{title}</div>
      <div className="text-[9px] font-mono text-gray-500 mt-1">{desc}</div>
    </button>
  );
}
