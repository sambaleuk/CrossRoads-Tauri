import React from 'react';

// PRD-05 will replace this with the full Dashboard layout
export default function App() {
  return (
    <div className="h-screen w-screen bg-gray-950 text-gray-100 flex items-center justify-center">
      <div className="text-center space-y-4">
        <div className="text-6xl">🧠</div>
        <h1 className="text-3xl font-bold font-mono">XRoads</h1>
        <p className="text-gray-400 text-sm font-mono">
          Multi-agent orchestration platform
        </p>
        <p className="text-gray-600 text-xs font-mono">
          Tauri + React + Rust — scaffold ready
        </p>
      </div>
    </div>
  );
}
