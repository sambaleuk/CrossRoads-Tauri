import { useState, useEffect } from 'react';
import * as api from '../services/api';
import { useAppStore } from '../stores/appStore';
import type { Workspace } from '../models';

/**
 * Workspace sidebar — switch between projects.
 * Shows as a narrow strip on the far left when multiple workspaces exist.
 */
export function WorkspaceSwitcher() {
  const { setProjectPath } = useAppStore();
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [showAdd, setShowAdd] = useState(false);
  const [newName, setNewName] = useState('');
  const [newPath, setNewPath] = useState('');

  useEffect(() => {
    loadWorkspaces();
  }, []);

  const loadWorkspaces = async () => {
    try { setWorkspaces(await api.fetchWorkspaces()); } catch {}
  };

  const switchTo = async (ws: Workspace) => {
    try {
      await api.switchWorkspace(ws.id);
      setProjectPath(ws.projectPath);
      await loadWorkspaces();
    } catch {}
  };

  const addWorkspace = async () => {
    if (!newName.trim() || !newPath.trim()) return;
    try {
      await api.createWorkspace(newName.trim(), newPath.trim());
      setNewName('');
      setNewPath('');
      setShowAdd(false);
      await loadWorkspaces();
    } catch {}
  };

  const removeWorkspace = async (id: string) => {
    try {
      await api.deleteWorkspace(id);
      await loadWorkspaces();
    } catch {}
  };

  // Don't render if no workspaces and not adding
  if (workspaces.length === 0 && !showAdd) {
    return (
      <div className="w-10 flex flex-col items-center py-2 border-r border-gray-800 bg-gray-950">
        <button
          onClick={() => setShowAdd(true)}
          title="Add workspace"
          className="w-7 h-7 rounded-lg border border-dashed border-gray-700 hover:border-neon-green/40 flex items-center justify-center text-gray-600 hover:text-neon-green transition-colors text-sm"
        >
          +
        </button>
      </div>
    );
  }

  return (
    <div className="w-12 flex flex-col items-center py-2 gap-2 border-r border-gray-800 bg-gray-950">
      {/* Workspace dots */}
      {workspaces.map((ws, i) => (
        <div key={ws.id} className="relative group">
          <button
            onClick={() => switchTo(ws)}
            onContextMenu={(e) => { e.preventDefault(); removeWorkspace(ws.id); }}
            title={`${ws.name}\n${ws.projectPath}\nRight-click to remove`}
            className={`w-8 h-8 rounded-lg flex items-center justify-center text-[11px] font-bold font-mono transition-all ${
              ws.isActive
                ? 'bg-neon-green/20 text-neon-green border border-neon-green/40 shadow-[0_0_8px_var(--xr-neon-green)]'
                : 'bg-gray-800/60 text-gray-400 border border-gray-700 hover:border-gray-600 hover:text-gray-200'
            }`}
            style={{ borderLeftColor: ws.isActive ? ws.color : undefined, borderLeftWidth: ws.isActive ? 3 : undefined }}
          >
            {ws.icon ?? ws.name.charAt(0).toUpperCase()}
          </button>

          {/* Active indicator bar */}
          {ws.isActive && (
            <div className="absolute -left-[7px] top-1 bottom-1 w-1 rounded-r bg-neon-green" />
          )}

          {/* Tooltip */}
          <div className="absolute left-full ml-2 top-0 hidden group-hover:block z-50">
            <div className="bg-gray-800 border border-gray-700 rounded px-2 py-1 whitespace-nowrap shadow-lg">
              <div className="text-[10px] font-bold font-mono text-gray-200">{ws.name}</div>
              <div className="text-[8px] font-mono text-gray-500">{ws.projectPath.split('/').pop()}</div>
            </div>
          </div>

          {/* Keyboard shortcut hint */}
          {i < 9 && (
            <div className="absolute -bottom-0.5 left-1/2 -translate-x-1/2 text-[7px] font-mono text-gray-700">
              ⌘{i + 1}
            </div>
          )}
        </div>
      ))}

      {/* Add button */}
      <button
        onClick={() => setShowAdd(!showAdd)}
        className="w-7 h-7 rounded-lg border border-dashed border-gray-700 hover:border-neon-green/40 flex items-center justify-center text-gray-600 hover:text-neon-green transition-colors text-xs mt-1"
      >
        +
      </button>

      {/* Add workspace popover */}
      {showAdd && (
        <div className="absolute left-14 top-16 z-50 bg-gray-900 border border-gray-700 rounded-lg shadow-2xl p-3 w-64 xr-animate-fade-in">
          <h3 className="text-[10px] font-bold font-mono text-gray-300 mb-2">Add Workspace</h3>
          <input
            type="text" value={newName} onChange={e => setNewName(e.target.value)}
            placeholder="Project name"
            className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-1 text-[11px] font-mono text-gray-200 placeholder:text-gray-600 focus:border-neon-green/40 focus:outline-none mb-2"
          />
          <input
            type="text" value={newPath} onChange={e => setNewPath(e.target.value)}
            placeholder="/path/to/project"
            className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-1 text-[11px] font-mono text-gray-200 placeholder:text-gray-600 focus:border-neon-green/40 focus:outline-none mb-2"
          />
          <div className="flex gap-2">
            <button onClick={() => setShowAdd(false)} className="text-[9px] font-mono text-gray-500 hover:text-gray-300">Cancel</button>
            <button onClick={addWorkspace}
              className="text-[9px] font-mono text-neon-green bg-neon-green/10 border border-neon-green/30 px-3 py-1 rounded hover:bg-neon-green/20">
              Add
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
