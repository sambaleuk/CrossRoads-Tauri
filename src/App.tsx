import { useEffect, useState, useRef } from 'react';
import { Toolbar, BottomBar } from './components/Toolbar';
import { Dashboard } from './views/Dashboard';
import { ChatPanel } from './views/ChatPanel';
import { GitPanel } from './views/GitPanel';
import { CockpitPanel } from './views/CockpitPanel';
import { CommandPalette, useCommandPalette } from './components/CommandPalette';
import { ReviewOverlay } from './components/ReviewOverlay';
import { useAppStore } from './stores/appStore';
import { initEventListeners } from './services/eventBus';

export default function App() {
  const { showCockpit, showInspector, showChat, session, slots, sessionCost } = useAppStore();
  const commandPalette = useCommandPalette();
  const [showReview, setShowReview] = useState(false);
  const prevSessionStatus = useRef(session?.status);

  // Initialize event listeners on mount
  useEffect(() => {
    initEventListeners();
  }, []);

  // Detect orchestration completion → show review overlay
  useEffect(() => {
    const prev = prevSessionStatus.current;
    const curr = session?.status;
    prevSessionStatus.current = curr;

    // Show review when: session transitions to closed, OR all slots are done
    if (prev === 'active' && curr === 'closed') {
      setShowReview(true);
    }
    if (slots.length > 0 && slots.every(s => s.status === 'done' || s.status === 'error') && slots.some(s => s.status === 'done')) {
      setShowReview(true);
    }
  }, [session?.status, slots]);

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

      {commandPalette.isOpen && <CommandPalette onClose={commandPalette.close} />}

      {showReview && session && (
        <ReviewOverlay
          sessionId={session.id}
          sessionCost={sessionCost}
          slots={slots.map(s => ({ agentType: s.agentType, status: s.status, currentTask: s.currentTask }))}
          startedAt={session.createdAt}
          onClose={() => setShowReview(false)}
        />
      )}
    </div>
  );
}
