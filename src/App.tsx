import { useEffect } from 'react';
import { Toolbar, BottomBar } from './components/Toolbar';
import { Dashboard } from './views/Dashboard';
import { ChatPanel } from './views/ChatPanel';
import { GitPanel } from './views/GitPanel';
import { CockpitPanel } from './views/CockpitPanel';
import { useAppStore } from './stores/appStore';
import { initEventListeners } from './services/eventBus';

export default function App() {
  const { showCockpit, showInspector, showChat } = useAppStore();

  // Initialize event listeners on mount
  useEffect(() => {
    initEventListeners();
  }, []);

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
        {showChat && <ChatPanel />}
        <Dashboard />
        {showCockpit && (
          <div className="w-80 border-l border-gray-800 flex flex-col bg-gray-950">
            <CockpitPanel />
          </div>
        )}
        {showInspector && <GitPanel />}
      </div>

      <BottomBar />
    </div>
  );
}
