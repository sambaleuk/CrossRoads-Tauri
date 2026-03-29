import { useEffect } from 'react';
import { Toolbar } from './components/Toolbar';
import { Dashboard } from './views/Dashboard';
import { ChatPanel } from './views/ChatPanel';
import { GitPanel } from './views/GitPanel';
import { CockpitPanel } from './views/CockpitPanel';
import { useAppStore } from './stores/appStore';

export default function App() {
  const { showCockpit, showInspector, showChat } = useAppStore();

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.shiftKey) {
        switch (e.key.toLowerCase()) {
          case 'c': e.preventDefault(); useAppStore.getState().toggleCockpit(); break;
          case 'i': e.preventDefault(); useAppStore.getState().toggleInspector(); break;
          case 'o': e.preventDefault(); useAppStore.getState().toggleChat(); break;
        }
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);

  return (
    <div className="h-screen w-screen bg-gray-950 text-gray-100 flex flex-col overflow-hidden">
      <Toolbar />

      <div className="flex flex-1 overflow-hidden">
        {/* Left: Chat panel */}
        {showChat && (
          <div className="w-72 border-r border-gray-800 flex flex-col bg-gray-950">
            <ChatPanel />
          </div>
        )}

        {/* Center: Terminal grid */}
        <Dashboard />

        {/* Right: Cockpit panel */}
        {showCockpit && (
          <div className="w-80 border-l border-gray-800 flex flex-col bg-gray-950">
            <CockpitPanel />
          </div>
        )}

        {/* Far right: Inspector */}
        {showInspector && (
          <div className="w-72 border-l border-gray-800 flex flex-col bg-gray-950">
            <GitPanel />
          </div>
        )}
      </div>
    </div>
  );
}
