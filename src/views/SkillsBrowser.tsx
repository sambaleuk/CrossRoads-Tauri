import { useState } from 'react';

interface Skill {
  name: string;
  family: string;
  description: string;
  requiredMcps: string[];
}

const builtinSkills: Skill[] = [
  { name: 'code-architect', family: 'dev', description: 'Architecture review and scaffolding', requiredMcps: [] },
  { name: 'feature-builder', family: 'dev', description: 'Implement core feature logic', requiredMcps: [] },
  { name: 'test-engineer', family: 'dev', description: 'Write comprehensive test suites', requiredMcps: [] },
  { name: 'finance-analyst', family: 'ops', description: 'Stripe reports + anomaly alerts + financial model', requiredMcps: ['Google Drive', 'Gmail'] },
  { name: 'legal-clerk', family: 'ops', description: 'Contracts MSA/SOW/NDA from templates', requiredMcps: ['Google Drive', 'Gmail', 'Notion'] },
  { name: 'hr-wiki-manager', family: 'ops', description: 'Onboarding packages + SOPs + wiki', requiredMcps: ['Notion', 'Gmail'] },
];

export function SkillsBrowser({ onClose }: { onClose: () => void }) {
  const [filter, setFilter] = useState<string>('all');
  const families = ['all', ...new Set(builtinSkills.map((s) => s.family))];
  const filtered = filter === 'all' ? builtinSkills : builtinSkills.filter((s) => s.family === filter);

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-gray-900 rounded-xl border border-gray-700 w-[500px] max-h-[450px] flex flex-col" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center gap-4 px-4 py-3 border-b border-gray-800">
          <h2 className="text-sm font-bold font-mono text-gray-200">Skills</h2>
          <div className="flex gap-1">
            {families.map((f) => (
              <button key={f} onClick={() => setFilter(f)}
                className={`px-2 py-0.5 rounded text-[9px] font-mono ${filter === f ? 'bg-neon-green/10 text-neon-green' : 'text-gray-500 hover:text-gray-300'}`}>
                {f}
              </button>
            ))}
          </div>
          <div className="flex-1" />
          <button onClick={onClose} className="text-gray-500 hover:text-gray-300 text-sm">✕</button>
        </div>

        <div className="flex-1 overflow-auto p-3 space-y-2">
          {filtered.map((skill) => (
            <div key={skill.name} className="p-3 rounded-lg bg-gray-800/40 border border-gray-800 hover:border-gray-700 transition-colors">
              <div className="flex items-center gap-2 mb-1">
                <span className="text-[11px] font-bold font-mono text-gray-200">{skill.name}</span>
                <span className="text-[8px] font-mono text-gray-500 bg-gray-800 px-1.5 py-0.5 rounded">{skill.family}</span>
              </div>
              <p className="text-[10px] font-mono text-gray-400">{skill.description}</p>
              {skill.requiredMcps.length > 0 && (
                <div className="flex gap-1 mt-1.5">
                  {skill.requiredMcps.map((mcp) => (
                    <span key={mcp} className="text-[8px] font-mono text-neon-purple bg-neon-purple/10 px-1.5 py-0.5 rounded">{mcp}</span>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
