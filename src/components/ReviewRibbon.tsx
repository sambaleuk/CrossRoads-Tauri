import { useState, useEffect, useCallback, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../stores/appStore';
import type { BrainProposal } from '../models';

type RibbonTab = 'proposals' | 'files' | 'diffs';

// ── Tree model ──

interface TreeNode {
  name: string;
  fullPath: string;
  relativePath: string;
  isDirectory: boolean;
  children: TreeNode[];
  expanded: boolean;
}

function buildTree(paths: { path: string; rel: string; isDir: boolean }[], rootName: string): TreeNode {
  const root: TreeNode = { name: rootName, fullPath: '', relativePath: '', isDirectory: true, children: [], expanded: true };

  for (const { path, rel, isDir } of paths) {
    const parts = rel.split('/');
    let current = root;
    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isLast = i === parts.length - 1;
      let child = current.children.find(c => c.name === part);
      if (!child) {
        child = {
          name: part,
          fullPath: isLast ? path : path.split('/').slice(0, -(parts.length - i - 1)).join('/'),
          relativePath: parts.slice(0, i + 1).join('/'),
          isDirectory: isLast ? isDir : true,
          children: [],
          expanded: false,
        };
        current.children.push(child);
      }
      if (isLast) {
        child.fullPath = path;
        child.isDirectory = isDir;
      }
      current = child;
    }
  }

  // Compact single-child directories
  function compact(node: TreeNode) {
    for (const child of node.children) compact(child);
    while (node.children.length === 1 && node.children[0].isDirectory && node.children[0].children.length > 0) {
      const only = node.children[0];
      node.name = node.name + '/' + only.name;
      node.fullPath = only.fullPath;
      node.relativePath = only.relativePath;
      node.children = only.children;
    }
  }
  compact(root);

  // Sort: dirs first, then alpha
  function sortTree(node: TreeNode) {
    node.children.sort((a, b) => {
      if (a.isDirectory !== b.isDirectory) return a.isDirectory ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
    for (const child of node.children) sortTree(child);
  }
  sortTree(root);

  // Auto-expand 2 levels
  function expandDepth(node: TreeNode, depth: number) {
    if (depth <= 0 || !node.isDirectory) return;
    node.expanded = true;
    for (const child of node.children) expandDepth(child, depth - 1);
  }
  expandDepth(root, 2);

  return root;
}

// ── Syntax Highlighting ──

type TokenType = 'keyword' | 'string' | 'comment' | 'number' | 'type' | 'annotation' | 'punctuation' | 'plain';

interface Token { text: string; type: TokenType; }

const KEYWORDS: Record<string, Set<string>> = {
  swift: new Set(['import','func','var','let','if','else','guard','return','for','while','switch','case','default','struct','class','enum','protocol','extension','private','public','internal','static','final','async','await','throws','throw','try','catch','do','in','where','self','Self','super','nil','true','false','some','any','typealias','init','override','mutating','weak','lazy','break','continue']),
  ts: new Set(['import','export','from','const','let','var','function','return','if','else','for','while','switch','case','default','break','continue','class','extends','implements','interface','type','enum','new','this','async','await','try','catch','throw','finally','typeof','instanceof','true','false','null','undefined','void','of','in','as','is','readonly','private','public','protected','static','abstract','super','yield']),
  py: new Set(['import','from','def','class','return','if','elif','else','for','while','try','except','finally','raise','with','as','pass','break','continue','and','or','not','in','is','None','True','False','lambda','yield','global','self','async','await']),
  rs: new Set(['fn','let','mut','if','else','match','for','while','loop','return','struct','enum','impl','trait','pub','use','mod','crate','self','super','async','await','move','ref','where','type','const','static','unsafe','true','false','as','in','dyn','extern','break','continue']),
  go: new Set(['package','import','func','var','const','type','struct','interface','if','else','for','range','switch','case','default','return','break','continue','go','defer','select','chan','map','nil','true','false','make','new','append','len','cap']),
};

const TYPE_KEYWORDS: Record<string, Set<string>> = {
  swift: new Set(['String','Int','Bool','Double','Float','Array','Dictionary','Set','Optional','Result','Error','URL','Data','Date','UUID','View','Color','CGFloat','Void','Any','Sendable','Codable','Hashable','Identifiable']),
  ts: new Set(['string','number','boolean','any','void','never','unknown','object','Array','Promise','Record','Partial','Required','Readonly','Map','Set']),
  rs: new Set(['String','Vec','Option','Result','Box','Rc','Arc','HashMap','i8','i16','i32','i64','u8','u16','u32','u64','f32','f64','bool','usize','isize','str','Self']),
  py: new Set(['int','str','float','bool','list','dict','tuple','set','Optional','List','Dict','Tuple','Set','Any','Union','Type']),
};

const SUPPORTED_LANGS = new Set(['swift','ts','tsx','js','jsx','py','rs','go','java','c','cpp','h','hpp','cs','rb','sh','bash','html','css','json','yml','yaml','toml','sql']);

function mapLang(ext: string): string {
  if (ext === 'tsx' || ext === 'jsx') return 'ts';
  if (ext === 'bash' || ext === 'sh') return 'sh';
  return ext;
}

function tokenizeLine(line: string, lang: string): Token[] {
  const kw = KEYWORDS[lang] ?? KEYWORDS['ts'] ?? new Set();
  const tw = TYPE_KEYWORDS[lang] ?? new Set();
  const tokens: Token[] = [];
  let i = 0;

  while (i < line.length) {
    // Comment
    if (line.startsWith('//', i) || ((lang === 'py' || lang === 'sh' || lang === 'yml' || lang === 'yaml') && line[i] === '#')) {
      tokens.push({ text: line.slice(i), type: 'comment' });
      break;
    }
    // String
    if (line[i] === '"' || line[i] === "'" || line[i] === '`') {
      const q = line[i];
      let j = i + 1;
      while (j < line.length) {
        if (line[j] === '\\') { j += 2; continue; }
        if (line[j] === q) { j++; break; }
        j++;
      }
      tokens.push({ text: line.slice(i, j), type: 'string' });
      i = j;
      continue;
    }
    // Number
    if (/[0-9]/.test(line[i]) || (line[i] === '.' && i + 1 < line.length && /[0-9]/.test(line[i + 1]))) {
      let j = i;
      while (j < line.length && /[0-9a-fA-Fx._]/.test(line[j])) j++;
      tokens.push({ text: line.slice(i, j), type: 'number' });
      i = j;
      continue;
    }
    // Annotation
    if (line[i] === '@') {
      let j = i + 1;
      while (j < line.length && /[a-zA-Z0-9_]/.test(line[j])) j++;
      tokens.push({ text: line.slice(i, j), type: 'annotation' });
      i = j;
      continue;
    }
    // Word
    if (/[a-zA-Z_]/.test(line[i])) {
      let j = i;
      while (j < line.length && /[a-zA-Z0-9_]/.test(line[j])) j++;
      const word = line.slice(i, j);
      if (kw.has(word)) tokens.push({ text: word, type: 'keyword' });
      else if (tw.has(word) || (word[0] === word[0].toUpperCase() && word.length > 1 && /[a-z]/.test(word))) tokens.push({ text: word, type: 'type' });
      else tokens.push({ text: word, type: 'plain' });
      i = j;
      continue;
    }
    // Punctuation
    tokens.push({ text: line[i], type: 'punctuation' });
    i++;
  }
  return tokens;
}

const TOKEN_COLORS: Record<TokenType, string> = {
  keyword: 'text-purple-400',
  string: 'text-orange-400',
  comment: 'text-gray-500 italic',
  number: 'text-yellow-300',
  type: 'text-cyan-400',
  annotation: 'text-amber-400',
  punctuation: 'text-gray-500',
  plain: 'text-gray-200',
};

// ── File icon helpers ──

function fileIconColor(ext: string): string {
  switch (ext) {
    case 'swift': return 'text-orange-400';
    case 'ts': case 'tsx': return 'text-blue-400';
    case 'js': case 'jsx': return 'text-yellow-400';
    case 'py': return 'text-blue-300';
    case 'rs': return 'text-orange-500';
    case 'md': return 'text-cyan-400';
    case 'json': case 'yml': case 'yaml': case 'toml': return 'text-green-400';
    default: return 'text-gray-500';
  }
}

// ── Main component ──

export function ReviewRibbon() {
  const { pendingProposals, removeProposal, setShowReviewRibbon, projectPath } = useAppStore();

  const [tab, setTab] = useState<RibbonTab>('proposals');
  const [tree, setTree] = useState<TreeNode | null>(null);
  const [selectedFile, setSelectedFile] = useState<{ path: string; rel: string; name: string } | null>(null);
  const [fileContent, setFileContent] = useState('');
  const [diffContent, setDiffContent] = useState('');
  const [searchText, setSearchText] = useState('');
  const [loadingFile, setLoadingFile] = useState(false);
  // Force re-render on expand/collapse
  const [, setTreeVersion] = useState(0);

  const proposals = useMemo(() =>
    Object.values(pendingProposals).sort((a, b) => new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime()),
    [pendingProposals]
  );

  useEffect(() => { if (proposals.length > 0) setTab('proposals'); }, [proposals.length]);

  useEffect(() => {
    if (!projectPath) return;
    loadFileTree(projectPath);
  }, [projectPath]);

  useEffect(() => {
    if (tab === 'diffs' && projectPath && !diffContent) loadDiff(projectPath);
  }, [tab, projectPath]);

  const loadFileTree = async (path: string) => {
    try {
      const result = await invoke<string>('run_shell_command', {
        command: `find "${path}" -maxdepth 4 -not -path '*/.*' -not -path '*/node_modules/*' -not -path '*/.build/*' -not -path '*/target/*' -not -path '*/__pycache__/*' 2>/dev/null | head -500`,
      }).catch(() => '');

      const lines = result.split('\n').filter(p => p && p !== path).sort();
      const items = lines.map(p => {
        const rel = p.slice(path.length + 1);
        // Simple heuristic: if it has an extension with a dot, it's a file
        const lastPart = p.split('/').pop() || '';
        const hasExt = lastPart.includes('.') && !lastPart.startsWith('.');
        return { path: p, rel, isDir: !hasExt };
      });

      const rootName = path.split('/').pop() || 'project';
      setTree(buildTree(items, rootName));
    } catch { /* optional */ }
  };

  const loadFileContent = async (path: string, rel: string, name: string) => {
    setLoadingFile(true);
    setSelectedFile({ path, rel, name });
    try {
      const content = await invoke<string>('run_shell_command', {
        command: `cat "${path}" 2>/dev/null | head -2000`,
      });
      setFileContent(content);
    } catch {
      setFileContent('Error reading file');
    }
    setLoadingFile(false);
  };

  const loadDiff = async (path: string) => {
    try {
      const diff = await invoke<string>('run_shell_command', {
        command: `cd "${path}" && git diff --stat --patch 2>/dev/null || echo "Not a git repository"`,
      });
      setDiffContent(diff);
    } catch {
      setDiffContent('Error running git diff');
    }
  };

  const handleApprove = useCallback(async (proposal: BrainProposal) => {
    removeProposal(proposal.id);
    if (proposal.proposalType === 'launch' && proposal.agentType && proposal.role && proposal.task) {
      try {
        await invoke('launch_slot_from_brain', {
          agentType: proposal.agentType, role: proposal.role, task: proposal.task, projectPath: projectPath || '',
        });
      } catch (e) { console.error('Failed to launch slot:', e); }
    } else if (proposal.proposalType === 'suite' && proposal.suiteId) {
      useAppStore.getState().setActiveSuiteId(proposal.suiteId);
    }
  }, [projectPath, removeProposal]);

  const handleReject = useCallback((p: BrainProposal) => removeProposal(p.id), [removeProposal]);

  const handleApproveAll = useCallback(async () => {
    for (const p of proposals) await handleApprove(p);
  }, [proposals, handleApprove]);

  const toggleNode = (node: TreeNode) => {
    node.expanded = !node.expanded;
    setTreeVersion(v => v + 1);
  };

  return (
    <div className="fixed inset-0 z-50 flex flex-col">
      <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" onClick={() => setShowReviewRibbon(false)} />

      <div className="relative m-4 flex flex-col flex-1 rounded-xl border border-gray-700/50 bg-gray-900/[0.98] shadow-2xl overflow-hidden animate-in slide-in-from-top duration-300">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-2 bg-gray-800/80 border-b border-gray-700/50">
          <div className="flex gap-1">
            {(['proposals', 'files', 'diffs'] as RibbonTab[]).map(t => (
              <button key={t} onClick={() => setTab(t)}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-[11px] font-mono font-medium transition-colors ${tab === t ? 'bg-gray-700 text-white' : 'text-gray-400 hover:text-gray-200 hover:bg-gray-700/50'}`}>
                <span className="text-[10px]">{t === 'proposals' ? '🧠' : t === 'files' ? '📁' : '↔'}</span>
                <span>{t.charAt(0).toUpperCase() + t.slice(1)}</span>
                {t === 'proposals' && proposals.length > 0 && (
                  <span className="ml-1 px-1.5 py-0.5 rounded-full bg-red-500 text-white text-[9px] font-bold">{proposals.length}</span>
                )}
              </button>
            ))}
          </div>
          <div className="flex items-center gap-2">
            {proposals.length > 0 && (
              <button onClick={handleApproveAll}
                className="flex items-center gap-1 px-3 py-1.5 rounded-md bg-green-600 hover:bg-green-500 text-white text-[11px] font-mono font-semibold transition-colors">
                ✓ Approve All ({proposals.length})
              </button>
            )}
            <button onClick={() => setShowReviewRibbon(false)} className="text-gray-400 hover:text-white text-lg leading-none">×</button>
          </div>
        </div>

        {/* Content */}
        <div className="flex flex-1 overflow-hidden">
          {/* Sidebar */}
          <div className="w-72 border-r border-gray-700/50 bg-gray-800/50 flex flex-col overflow-hidden">
            {tab === 'proposals' ? (
              <ProposalsSidebar proposals={proposals} />
            ) : (
              <div className="flex flex-col h-full">
                <div className="p-2">
                  <input type="text" value={searchText} onChange={e => setSearchText(e.target.value)} placeholder="Filter files..."
                    className="w-full px-2 py-1 rounded-md bg-gray-700/60 border border-gray-600/40 text-[11px] font-mono text-gray-200 placeholder-gray-500 focus:outline-none focus:border-cyan-500/50" />
                </div>
                <div className="flex-1 overflow-auto px-1">
                  {tree && tree.children.map(child => (
                    <TreeNodeRow key={child.relativePath} node={child} depth={0} filter={searchText}
                      selectedPath={selectedFile?.path}
                      onToggle={toggleNode}
                      onSelect={(n) => loadFileContent(n.fullPath, n.relativePath, n.name)} />
                  ))}
                </div>
              </div>
            )}
          </div>

          {/* Main */}
          <div className="flex-1 overflow-auto">
            {tab === 'proposals' && <ProposalsContent proposals={proposals} onApprove={handleApprove} onReject={handleReject} />}
            {tab === 'files' && <FileViewer file={selectedFile} content={fileContent} loading={loadingFile} />}
            {tab === 'diffs' && <DiffViewer diff={diffContent} />}
          </div>
        </div>
      </div>
    </div>
  );
}

// ── TreeNodeRow (recursive, collapsible) ──

function TreeNodeRow({ node, depth, filter, selectedPath, onToggle, onSelect }: {
  node: TreeNode; depth: number; filter: string; selectedPath?: string;
  onToggle: (n: TreeNode) => void; onSelect: (n: TreeNode) => void;
}) {
  const matches = filter ? matchesFilter(node, filter) : true;
  if (!matches) return null;

  const ext = node.name.split('.').pop()?.toLowerCase() || '';
  const isSelected = !node.isDirectory && node.fullPath === selectedPath;

  return (
    <>
      <button
        onClick={() => node.isDirectory ? onToggle(node) : onSelect(node)}
        className={`w-full flex items-center gap-1 py-[2px] rounded text-left text-[11px] font-mono transition-colors ${isSelected ? 'bg-gray-600/60 text-white' : 'text-gray-400 hover:text-gray-200 hover:bg-gray-700/30'}`}
        style={{ paddingLeft: `${8 + depth * 14}px` }}
      >
        {node.isDirectory ? (
          <span className="text-[8px] text-gray-500 w-[10px] text-center font-bold">{node.expanded ? '▾' : '▸'}</span>
        ) : (
          <span className="w-[10px]" />
        )}
        <span className={`text-[10px] ${node.isDirectory ? 'text-yellow-400' : fileIconColor(ext)}`}>
          {node.isDirectory ? (node.expanded ? '📂' : '📁') : '📄'}
        </span>
        <span className="truncate flex-1">{node.name}</span>
        {node.isDirectory && node.children.length > 0 && (
          <span className="text-[8px] text-gray-600 mr-1">{node.children.length}</span>
        )}
      </button>
      {node.isDirectory && node.expanded && node.children.map(child => (
        <TreeNodeRow key={child.relativePath} node={child} depth={depth + 1} filter={filter}
          selectedPath={selectedPath} onToggle={onToggle} onSelect={onSelect} />
      ))}
    </>
  );
}

function matchesFilter(node: TreeNode, filter: string): boolean {
  if (node.name.toLowerCase().includes(filter.toLowerCase())) return true;
  return node.children.some(c => matchesFilter(c, filter));
}

// ── ProposalsSidebar ──

function ProposalsSidebar({ proposals }: { proposals: BrainProposal[] }) {
  return (
    <div className="flex flex-col h-full">
      <div className="px-3 pt-2 pb-1 text-[9px] font-bold font-mono text-gray-500 uppercase tracking-wider">Pending Actions</div>
      {proposals.length === 0 ? (
        <div className="flex-1 flex flex-col items-center justify-center text-gray-500">
          <span className="text-2xl mb-2">✓</span>
          <span className="text-[11px] font-mono">No pending proposals</span>
        </div>
      ) : (
        <div className="flex-1 overflow-auto px-2 py-1 space-y-1">
          {proposals.map(p => (
            <div key={p.id} className="flex items-center gap-2 px-2 py-1.5 rounded-md bg-gray-700/40 hover:bg-gray-700/60">
              <span className={`w-1.5 h-1.5 rounded-full ${riskBg(p.riskLevel)}`} />
              <div className="flex-1 min-w-0">
                <div className="text-[10px] font-mono font-medium text-gray-200 truncate">{p.title}</div>
                <div className="text-[8px] font-mono text-gray-500 uppercase">{p.proposalType}</div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function riskBg(level: string) {
  switch (level) { case 'low': return 'bg-green-400'; case 'medium': return 'bg-yellow-400'; case 'high': return 'bg-orange-400'; case 'critical': return 'bg-red-400'; default: return 'bg-gray-400'; }
}

// ── ProposalsContent ──

function ProposalsContent({ proposals, onApprove, onReject }: { proposals: BrainProposal[]; onApprove: (p: BrainProposal) => void; onReject: (p: BrainProposal) => void }) {
  if (proposals.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-gray-500">
        <span className="text-4xl mb-3 opacity-40">🧠</span>
        <span className="text-[13px] font-mono font-medium">Brain is thinking...</span>
        <span className="text-[11px] font-mono mt-1 text-gray-600">Proposals will appear here when the brain recommends actions.</span>
      </div>
    );
  }
  return (
    <div className="p-4 space-y-4">
      {proposals.map(p => <ProposalCard key={p.id} proposal={p} onApprove={onApprove} onReject={onReject} />)}
    </div>
  );
}

function ProposalCard({ proposal, onApprove, onReject }: { proposal: BrainProposal; onApprove: (p: BrainProposal) => void; onReject: (p: BrainProposal) => void }) {
  const rc = riskColors(proposal.riskLevel);
  const icon = proposal.proposalType === 'launch' ? '▶' : proposal.proposalType === 'suite' ? '◆' : proposal.proposalType === 'alert' ? '⚠' : '🧠';
  const ago = timeAgo(proposal.createdAt);

  return (
    <div className={`rounded-lg border ${rc.border} ${rc.bg} p-4 shadow-lg ${rc.shadow}`}>
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <span className={`text-sm ${rc.text}`}>{icon}</span>
          <span className="text-[11px] font-bold font-mono text-gray-200 uppercase">{proposal.proposalType}</span>
        </div>
        <div className="flex items-center gap-2">
          <span className={`px-2 py-0.5 rounded-full text-[9px] font-bold font-mono uppercase ${rc.text} ${rc.bg}`}>{proposal.riskLevel}</span>
          <span className="text-[9px] font-mono text-gray-500">{ago}</span>
        </div>
      </div>
      <div className="border-t border-gray-700/50 my-2" />
      <div className="text-[13px] font-semibold font-mono text-gray-100 mb-2">{proposal.title}</div>
      <div className="px-3 py-2 rounded-md bg-gray-800/60 mb-3">
        <span className="text-[11px] font-mono text-cyan-400">{proposal.detail}</span>
      </div>
      {proposal.proposalType === 'launch' && (
        <div className="flex gap-4 mb-3">
          {proposal.agentType && <LabelValue label="Agent" value={proposal.agentType} />}
          {proposal.role && <LabelValue label="Role" value={proposal.role} />}
        </div>
      )}
      {proposal.scannerExcerpt && (
        <div className="mb-3">
          <div className="text-[9px] font-mono text-gray-500 mb-1">Scanner Context</div>
          <div className="text-[10px] font-mono text-gray-400 px-2 py-1 bg-gray-800/40 rounded line-clamp-5">{proposal.scannerExcerpt}</div>
        </div>
      )}
      <div className="border-t border-gray-700/50 my-2" />
      <div className="flex gap-2">
        <button onClick={() => onReject(proposal)} className="flex-1 flex items-center justify-center gap-1 py-1.5 rounded-md bg-red-600 hover:bg-red-500 text-white text-[11px] font-mono font-semibold transition-colors">✕ Reject</button>
        <button onClick={() => onApprove(proposal)} className="flex-1 flex items-center justify-center gap-1 py-1.5 rounded-md bg-green-600 hover:bg-green-500 text-white text-[11px] font-mono font-semibold transition-colors">✓ Approve</button>
      </div>
    </div>
  );
}

function LabelValue({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center gap-1">
      <span className="text-[9px] font-mono text-gray-500">{label}:</span>
      <span className="text-[10px] font-mono font-semibold text-gray-200 px-1.5 py-0.5 bg-gray-700/60 rounded">{value}</span>
    </div>
  );
}

function riskColors(level: string) {
  switch (level) {
    case 'low': return { border: 'border-green-500/40', bg: 'bg-green-500/10', text: 'text-green-400', shadow: 'shadow-green-500/10' };
    case 'medium': return { border: 'border-yellow-500/40', bg: 'bg-yellow-500/10', text: 'text-yellow-400', shadow: 'shadow-yellow-500/10' };
    case 'high': return { border: 'border-orange-500/40', bg: 'bg-orange-500/10', text: 'text-orange-400', shadow: 'shadow-orange-500/10' };
    case 'critical': return { border: 'border-red-500/40', bg: 'bg-red-500/10', text: 'text-red-400', shadow: 'shadow-red-500/10' };
    default: return { border: 'border-gray-500/40', bg: 'bg-gray-500/10', text: 'text-gray-400', shadow: '' };
  }
}

function timeAgo(dateStr: string) {
  const s = Math.floor((Date.now() - new Date(dateStr).getTime()) / 1000);
  if (s < 60) return `${s}s ago`;
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  return `${Math.floor(s / 3600)}h ago`;
}

// ── FileViewer with rendering engine ──

function FileViewer({ file, content, loading }: { file: { path: string; rel: string; name: string } | null; content: string; loading: boolean }) {
  if (loading) return <div className="flex items-center justify-center h-full text-gray-500 text-[11px] font-mono">Loading...</div>;
  if (!file) return (
    <div className="flex flex-col items-center justify-center h-full text-gray-500">
      <span className="text-3xl mb-2 opacity-30">📄</span>
      <span className="text-[12px] font-mono">Select a file to view</span>
    </div>
  );

  const ext = file.name.split('.').pop()?.toLowerCase() || '';
  const lineCount = content.split('\n').length;

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-2 px-4 py-2 bg-gray-800/60 border-b border-gray-700/50">
        <span className={`text-[10px] ${fileIconColor(ext)}`}>📄</span>
        <span className="text-[11px] font-mono font-medium text-gray-300">{file.rel}</span>
        <span className="text-[9px] font-mono text-gray-600 ml-auto">{lineCount} lines</span>
      </div>
      <div className="flex-1 overflow-auto p-4">
        {ext === 'md' || ext === 'markdown' ? (
          <MarkdownRenderer content={content} />
        ) : SUPPORTED_LANGS.has(ext) ? (
          <SyntaxHighlightedCode content={content} lang={mapLang(ext)} />
        ) : (
          <PlainCodeWithLineNumbers content={content} />
        )}
      </div>
    </div>
  );
}

// ── PlainCodeWithLineNumbers ──

function PlainCodeWithLineNumbers({ content }: { content: string }) {
  const lines = content.split('\n');
  const gw = String(lines.length).length * 8 + 16;
  return (
    <div className="select-text">
      {lines.map((line, i) => (
        <div key={i} className="flex">
          <span className="text-[11px] font-mono text-gray-600/40 select-none" style={{ width: gw, textAlign: 'right', paddingRight: 8 }}>{i + 1}</span>
          <span className="text-[12px] font-mono text-gray-200 whitespace-pre">{line || ' '}</span>
        </div>
      ))}
    </div>
  );
}

// ── SyntaxHighlightedCode ──

function SyntaxHighlightedCode({ content, lang }: { content: string; lang: string }) {
  const lines = content.split('\n');
  const gw = String(lines.length).length * 8 + 16;
  return (
    <div className="select-text">
      {lines.map((line, i) => {
        const tokens = tokenizeLine(line, lang);
        return (
          <div key={i} className="flex">
            <span className="text-[11px] font-mono text-gray-600/35 select-none" style={{ width: gw, textAlign: 'right', paddingRight: 8 }}>{i + 1}</span>
            <span className="text-[12px] font-mono whitespace-pre">
              {tokens.length === 0 ? ' ' : tokens.map((t, j) => (
                <span key={j} className={TOKEN_COLORS[t.type]}>{t.text}</span>
              ))}
            </span>
          </div>
        );
      })}
    </div>
  );
}

// ── MarkdownRenderer ──

function MarkdownRenderer({ content }: { content: string }) {
  // Pre-parse into blocks (code blocks collapsed)
  const blocks: Array<{ type: 'line'; text: string } | { type: 'code'; code: string; lang: string }> = [];
  const lines = content.split('\n');
  let inCode = false, codeLines: string[] = [], codeLang = '';

  for (const line of lines) {
    if (line.startsWith('```')) {
      if (inCode) {
        blocks.push({ type: 'code', code: codeLines.join('\n'), lang: codeLang });
        codeLines = []; codeLang = '';
      } else {
        codeLang = line.slice(3).trim();
      }
      inCode = !inCode;
    } else if (inCode) {
      codeLines.push(line);
    } else {
      blocks.push({ type: 'line', text: line });
    }
  }
  if (codeLines.length) blocks.push({ type: 'code', code: codeLines.join('\n'), lang: codeLang });

  return (
    <div className="space-y-1 select-text">
      {blocks.map((block, i) => {
        if (block.type === 'code') {
          return (
            <div key={i} className="rounded-md border border-gray-700/50 bg-gray-800/60 overflow-hidden my-2">
              {block.lang && <div className="text-[9px] font-bold font-mono text-gray-500 px-3 py-1 bg-gray-800">{block.lang}</div>}
              <div className="p-3">
                {SUPPORTED_LANGS.has(mapLang(block.lang)) ? (
                  <SyntaxHighlightedCode content={block.code} lang={mapLang(block.lang)} />
                ) : (
                  <pre className="text-[11px] font-mono text-gray-200 whitespace-pre-wrap">{block.code}</pre>
                )}
              </div>
            </div>
          );
        }
        const line = block.text;
        if (line.startsWith('# ')) return <div key={i} className="text-[18px] font-bold text-gray-100 pt-3 pb-1">{inlineMarkdown(line.slice(2))}</div>;
        if (line.startsWith('## ')) return <div key={i} className="text-[15px] font-bold text-gray-100 pt-2 pb-0.5">{inlineMarkdown(line.slice(3))}</div>;
        if (line.startsWith('### ')) return <div key={i} className="text-[13px] font-semibold text-gray-200 pt-1">{inlineMarkdown(line.slice(4))}</div>;
        if (line.startsWith('- ') || line.startsWith('* ')) return (
          <div key={i} className="flex gap-2 pl-2">
            <span className="text-cyan-400 text-[12px]">•</span>
            <span className="text-[12px] text-gray-300">{inlineMarkdown(line.slice(2))}</span>
          </div>
        );
        if (line.startsWith('> ')) return (
          <div key={i} className="border-l-2 border-cyan-500/40 pl-3 text-[12px] text-gray-400 italic">{inlineMarkdown(line.slice(2))}</div>
        );
        if (line.startsWith('---') || line.startsWith('***')) return <hr key={i} className="border-gray-700/50 my-2" />;
        if (line.trim() === '') return <div key={i} className="h-1.5" />;
        return <div key={i} className="text-[12px] text-gray-300">{inlineMarkdown(line)}</div>;
      })}
    </div>
  );
}

/** Inline markdown: **bold**, *italic*, `code` */
function inlineMarkdown(text: string): React.ReactElement {
  const parts: React.ReactElement[] = [];
  let remaining = text;
  let key = 0;

  while (remaining.length > 0) {
    // Inline code
    const codeMatch = remaining.match(/^`([^`]+)`/);
    if (codeMatch) {
      parts.push(<code key={key++} className="text-[11px] font-mono text-cyan-400 bg-gray-800 px-1 rounded">{codeMatch[1]}</code>);
      remaining = remaining.slice(codeMatch[0].length);
      continue;
    }
    // Bold
    const boldMatch = remaining.match(/^\*\*(.+?)\*\*/);
    if (boldMatch) {
      parts.push(<strong key={key++} className="font-bold text-gray-100">{boldMatch[1]}</strong>);
      remaining = remaining.slice(boldMatch[0].length);
      continue;
    }
    // Italic
    const italicMatch = remaining.match(/^\*(.+?)\*/);
    if (italicMatch) {
      parts.push(<em key={key++} className="italic text-gray-300">{italicMatch[1]}</em>);
      remaining = remaining.slice(italicMatch[0].length);
      continue;
    }
    // Plain char
    const nextSpecial = remaining.slice(1).search(/[`*]/);
    const chunk = nextSpecial === -1 ? remaining : remaining.slice(0, nextSpecial + 1);
    parts.push(<span key={key++}>{chunk}</span>);
    remaining = remaining.slice(chunk.length);
  }

  return <>{parts}</>;
}

// ── DiffViewer ──

function DiffViewer({ diff }: { diff: string }) {
  if (!diff || diff === 'Not a git repository') {
    return (
      <div className="flex flex-col items-center justify-center h-full text-gray-500">
        <span className="text-3xl mb-2 opacity-30">↔</span>
        <span className="text-[12px] font-mono">No changes detected</span>
      </div>
    );
  }

  const lines = diff.split('\n');
  const gw = String(lines.length).length * 8 + 16;

  return (
    <div className="p-4 select-text">
      {lines.map((line, i) => (
        <div key={i} className={`flex py-[1px] px-1 ${diffLineBg(line)}`}>
          <span className="text-[10px] font-mono text-gray-600/30 select-none" style={{ width: gw, textAlign: 'right', paddingRight: 6 }}>{i + 1}</span>
          <span className={`text-[11px] font-mono ${diffLineColor(line)}`}>{line || ' '}</span>
        </div>
      ))}
    </div>
  );
}

function diffLineColor(line: string) {
  if (line.startsWith('+++') || line.startsWith('---')) return 'text-gray-400';
  if (line.startsWith('@@')) return 'text-cyan-400';
  if (line.startsWith('+')) return 'text-green-400';
  if (line.startsWith('-')) return 'text-red-400';
  if (line.startsWith('diff ')) return 'text-yellow-400';
  return 'text-gray-300';
}

function diffLineBg(line: string) {
  if (line.startsWith('+') && !line.startsWith('+++')) return 'bg-green-500/5';
  if (line.startsWith('-') && !line.startsWith('---')) return 'bg-red-500/5';
  return '';
}
