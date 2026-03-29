import { useState, useEffect } from 'react';
import * as api from '../services/api';
import type { AgentRuntimeConfig } from '../models';

type SettingsTab = 'general' | 'cli' | 'api' | 'budget' | 'runtimes' | 'advanced';

interface CliTool {
  name: string;
  available: boolean;
  version?: string;
  path?: string;
}

// Persisted settings (localStorage)
function loadSettings(): Record<string, string> {
  try { return JSON.parse(localStorage.getItem('xroads-settings') ?? '{}'); } catch { return {}; }
}
function saveSetting(key: string, value: string) {
  const s = loadSettings();
  s[key] = value;
  localStorage.setItem('xroads-settings', JSON.stringify(s));
}

export function SettingsPanel({ onClose }: { onClose: () => void }) {
  const [tab, setTab] = useState<SettingsTab>('general');
  const [tools, setTools] = useState<CliTool[]>([]);
  const [runtimes, setRuntimes] = useState<AgentRuntimeConfig[]>([]);
  const settings = loadSettings();

  useEffect(() => {
    api.detectCliTools().then(setTools).catch(() => {});
    api.fetchRuntimes().then(setRuntimes).catch(() => {});
  }, []);

  const tabs: { key: SettingsTab; label: string }[] = [
    { key: 'general', label: 'General' },
    { key: 'cli', label: 'CLI' },
    { key: 'api', label: 'API Keys' },
    { key: 'budget', label: 'Budget' },
    { key: 'runtimes', label: 'Runtimes' },
    { key: 'advanced', label: 'Advanced' },
  ];

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-gray-900 rounded-xl border border-gray-700 w-[680px] max-h-[600px] flex flex-col xr-animate-scale-in" onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div className="flex items-center gap-4 px-4 py-3 border-b border-gray-800">
          <h2 className="text-sm font-bold font-mono text-gray-200">Settings</h2>
          <div className="flex-1" />
          <button onClick={onClose} className="text-gray-500 hover:text-gray-300 text-lg">×</button>
        </div>

        {/* Tabs */}
        <div className="flex border-b border-gray-800 overflow-x-auto">
          {tabs.map((t) => (
            <button key={t.key} onClick={() => setTab(t.key)}
              className={`px-3 py-2 text-[10px] font-mono whitespace-nowrap transition-colors ${
                tab === t.key ? 'text-neon-green border-b-2 border-neon-green' : 'text-gray-500 hover:text-gray-300'
              }`}>
              {t.label.toUpperCase()}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="flex-1 overflow-auto p-4">
          {tab === 'general' && <GeneralTab settings={settings} />}
          {tab === 'cli' && <CliTab tools={tools} settings={settings} />}
          {tab === 'api' && <ApiKeysTab settings={settings} />}
          {tab === 'budget' && <BudgetTab settings={settings} />}
          {tab === 'runtimes' && <RuntimesTab runtimes={runtimes} onRefresh={() => api.fetchRuntimes().then(setRuntimes).catch(() => {})} />}
          {tab === 'advanced' && <AdvancedTab settings={settings} />}
        </div>
      </div>
    </div>
  );
}

// ── General Tab ──

function GeneralTab({ settings }: { settings: Record<string, string> }) {
  const [repoPath, setRepoPath] = useState(settings['defaultRepoPath'] ?? '');
  const [maxLogs, setMaxLogs] = useState(settings['maxLogEntries'] ?? '5000');
  const [autoStart, setAutoStart] = useState(settings['autoStartLogStreaming'] === 'true');
  const [notifications, setNotifications] = useState(settings['enableNotifications'] !== 'false');

  const save = (key: string, value: string) => saveSetting(key, value);

  return (
    <div className="space-y-5">
      <SettingsSection title="Project">
        <SettingsField label="Default Repository Path">
          <input value={repoPath} onChange={(e) => { setRepoPath(e.target.value); save('defaultRepoPath', e.target.value); }}
            placeholder="/path/to/your/repo" className="settings-input" />
        </SettingsField>
      </SettingsSection>

      <SettingsSection title="Behavior">
        <SettingsToggle label="Auto-start Log Streaming" checked={autoStart}
          onChange={(v) => { setAutoStart(v); save('autoStartLogStreaming', String(v)); }} />
        <SettingsField label="Max Log Entries">
          <select value={maxLogs} onChange={(e) => { setMaxLogs(e.target.value); save('maxLogEntries', e.target.value); }} className="settings-select">
            <option value="1000">1,000</option>
            <option value="2500">2,500</option>
            <option value="5000">5,000</option>
            <option value="10000">10,000</option>
          </select>
        </SettingsField>
        <SettingsToggle label="Enable Notifications" checked={notifications}
          onChange={(v) => { setNotifications(v); save('enableNotifications', String(v)); }} />
      </SettingsSection>

      <SettingsSection title="Keyboard Shortcuts">
        <ShortcutRow label="Toggle Cockpit" shortcut="⌘⇧C" />
        <ShortcutRow label="Toggle Inspector" shortcut="⌘⇧I" />
        <ShortcutRow label="Toggle Chat" shortcut="⌘⇧O" />
        <ShortcutRow label="Command Palette" shortcut="⌘K" />
        <ShortcutRow label="Switch Workspace" shortcut="⌘1-9" />
      </SettingsSection>
    </div>
  );
}

// ── CLI Tab ──

function CliTab({ tools, settings }: { tools: CliTool[]; settings: Record<string, string> }) {
  return (
    <div className="space-y-5">
      <SettingsSection title="Detected CLI Tools">
        {tools.map((tool) => (
          <div key={tool.name} className="flex items-center gap-3 p-2.5 bg-gray-800/40 rounded-lg">
            <span className={`w-2.5 h-2.5 rounded-full shadow-sm ${tool.available ? 'bg-emerald-400 shadow-emerald-400/50' : 'bg-red-400 shadow-red-400/50'}`} />
            <span className="text-[11px] font-bold font-mono text-gray-200 w-16">{tool.name}</span>
            <span className="text-[10px] font-mono text-gray-400 flex-1 truncate">{tool.version ?? 'not installed'}</span>
            <span className="text-[9px] font-mono text-gray-600 truncate max-w-48">{tool.path ?? ''}</span>
            {tool.available && (
              <span className="text-[8px] font-mono text-emerald-400 bg-emerald-400/10 px-1.5 py-0.5 rounded">OK</span>
            )}
          </div>
        ))}
      </SettingsSection>

      <SettingsSection title="CLI Paths (Override)">
        <SettingsField label="Claude Code Path">
          <input defaultValue={settings['claudeCliPath'] ?? ''} onBlur={(e) => saveSetting('claudeCliPath', e.target.value)}
            placeholder="auto-detect" className="settings-input" />
        </SettingsField>
        <SettingsField label="Gemini CLI Path">
          <input defaultValue={settings['geminiCliPath'] ?? ''} onBlur={(e) => saveSetting('geminiCliPath', e.target.value)}
            placeholder="auto-detect" className="settings-input" />
        </SettingsField>
        <SettingsField label="Codex CLI Path">
          <input defaultValue={settings['codexCliPath'] ?? ''} onBlur={(e) => saveSetting('codexCliPath', e.target.value)}
            placeholder="auto-detect" className="settings-input" />
        </SettingsField>
      </SettingsSection>
    </div>
  );
}

// ── API Keys Tab ──

const API_PROVIDERS = [
  { key: 'anthropic', label: 'Anthropic', prefix: 'sk-ant-', placeholder: 'sk-ant-api03-...', docsUrl: 'https://console.anthropic.com/settings/keys' },
  { key: 'openai', label: 'OpenAI', prefix: 'sk-', placeholder: 'sk-proj-...', docsUrl: 'https://platform.openai.com/api-keys' },
  { key: 'google', label: 'Google AI', prefix: 'AIza', placeholder: 'AIzaSy...', docsUrl: 'https://aistudio.google.com/app/apikey' },
];

function ApiKeysTab({ settings }: { settings: Record<string, string> }) {
  return (
    <div className="space-y-5">
      <SettingsSection title="API Keys">
        <p className="text-[9px] font-mono text-gray-500 mb-3">Keys are stored in localStorage. For production, use environment variables.</p>
        {API_PROVIDERS.map((provider) => (
          <div key={provider.key} className="space-y-1">
            <div className="flex items-center gap-2">
              <span className="text-[10px] font-mono text-gray-400">{provider.label}</span>
              <a href={provider.docsUrl} target="_blank" rel="noreferrer" className="text-[8px] font-mono text-neon-blue hover:underline">Get key →</a>
            </div>
            <input type="password" defaultValue={settings[`${provider.key}ApiKey`] ?? ''}
              onBlur={(e) => saveSetting(`${provider.key}ApiKey`, e.target.value)}
              placeholder={provider.placeholder} className="settings-input" />
            {settings[`${provider.key}ApiKey`] && (
              <span className="text-[8px] font-mono text-emerald-400">✓ Key configured</span>
            )}
          </div>
        ))}
      </SettingsSection>
    </div>
  );
}

// ── Budget Tab (Phase 5) ──

const BUDGET_PRESETS = [
  { name: 'Frugal', cents: 500, desc: '$5 cap, Sonnet-only recommended' },
  { name: 'Standard', cents: 2500, desc: '$25 cap, mixed models' },
  { name: 'Performance', cents: 10000, desc: '$100 cap, Opus allowed' },
  { name: 'Unlimited', cents: 0, desc: 'No caps, monitoring only' },
];

function BudgetTab({ settings }: { settings: Record<string, string> }) {
  const [preset, setPreset] = useState(settings['budgetPreset'] ?? 'Standard');
  const [warningPct, setWarningPct] = useState(settings['budgetWarningPct'] ?? '80');
  const [hardStop, setHardStop] = useState(settings['budgetHardStop'] !== 'false');
  const [throttle, setThrottle] = useState(settings['budgetThrottle'] !== 'false');
  const [dailyLimit, setDailyLimit] = useState(settings['budgetDailyLimit'] ?? '');

  return (
    <div className="space-y-5">
      <SettingsSection title="Budget Preset">
        <div className="grid grid-cols-2 gap-2">
          {BUDGET_PRESETS.map((p) => (
            <button key={p.name} onClick={() => { setPreset(p.name); saveSetting('budgetPreset', p.name); }}
              className={`p-3 rounded-lg border text-left transition-colors ${
                preset === p.name ? 'border-neon-green/50 bg-neon-green/10' : 'border-gray-700 bg-gray-800/40 hover:border-gray-600'
              }`}>
              <div className="text-[11px] font-bold font-mono text-gray-200">{p.name}</div>
              <div className="text-[9px] font-mono text-gray-500">{p.desc}</div>
            </button>
          ))}
        </div>
      </SettingsSection>

      <SettingsSection title="Thresholds">
        <SettingsField label={`Warning at ${warningPct}%`}>
          <input type="range" min="50" max="95" step="5" value={warningPct}
            onChange={(e) => { setWarningPct(e.target.value); saveSetting('budgetWarningPct', e.target.value); }}
            className="w-full accent-neon-green" />
        </SettingsField>
        <SettingsToggle label="Hard Stop at 100% (auto-pause agent)" checked={hardStop}
          onChange={(v) => { setHardStop(v); saveSetting('budgetHardStop', String(v)); }} />
        <SettingsToggle label="Auto-Throttle (reduce speed at warning)" checked={throttle}
          onChange={(v) => { setThrottle(v); saveSetting('budgetThrottle', String(v)); }} />
        <SettingsField label="Daily Limit (cents, empty = none)">
          <input type="number" value={dailyLimit} onChange={(e) => { setDailyLimit(e.target.value); saveSetting('budgetDailyLimit', e.target.value); }}
            placeholder="No daily limit" className="settings-input" />
        </SettingsField>
      </SettingsSection>
    </div>
  );
}

// ── Runtimes Tab (Phase 5) ──

function RuntimesTab({ runtimes, onRefresh }: { runtimes: AgentRuntimeConfig[]; onRefresh: () => void }) {
  const [showAdd, setShowAdd] = useState(false);
  const [newName, setNewName] = useState('');
  const [newType, setNewType] = useState('cli');
  const [newCommand, setNewCommand] = useState('');
  const [newCaps, setNewCaps] = useState('["code_edit","test_run"]');

  const handleAdd = async () => {
    if (!newName.trim()) return;
    try {
      await api.registerRuntime(newName, newType, newCommand || undefined, undefined, newCaps);
      setShowAdd(false);
      setNewName('');
      setNewCommand('');
      onRefresh();
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <div className="space-y-5">
      <SettingsSection title="Agent Runtimes">
        <div className="flex justify-between items-center mb-2">
          <span className="text-[9px] font-mono text-gray-500">{runtimes.length} registered</span>
          <button onClick={() => setShowAdd(!showAdd)}
            className="text-[10px] font-mono text-neon-green hover:text-neon-green/80 transition-colors">
            + Add Runtime
          </button>
        </div>
        {runtimes.map((rt) => (
          <div key={rt.id} className="flex items-center gap-3 p-2.5 bg-gray-800/40 rounded-lg">
            <span className={`w-2 h-2 rounded-full ${rt.isEnabled ? 'bg-emerald-400' : 'bg-gray-600'}`} />
            <span className="text-[11px] font-bold font-mono text-gray-200 w-24">{rt.name}</span>
            <span className="text-[9px] font-mono text-neon-purple bg-neon-purple/10 px-1.5 py-0.5 rounded">{rt.runtimeType}</span>
            {rt.isBuiltin && <span className="text-[8px] font-mono text-gray-500 bg-gray-700 px-1 py-0.5 rounded">built-in</span>}
            <span className="text-[9px] font-mono text-gray-500 flex-1 truncate">{rt.command ?? rt.url ?? ''}</span>
          </div>
        ))}
      </SettingsSection>

      {showAdd && (
        <SettingsSection title="New Runtime">
          <SettingsField label="Name">
            <input value={newName} onChange={(e) => setNewName(e.target.value)} placeholder="My Python Agent" className="settings-input" />
          </SettingsField>
          <SettingsField label="Type">
            <select value={newType} onChange={(e) => setNewType(e.target.value)} className="settings-select">
              <option value="cli">CLI</option>
              <option value="http">HTTP Webhook</option>
              <option value="docker">Docker</option>
              <option value="script">Script</option>
              <option value="stdio">Stdin/Stdout</option>
            </select>
          </SettingsField>
          <SettingsField label="Command / URL">
            <input value={newCommand} onChange={(e) => setNewCommand(e.target.value)}
              placeholder={newType === 'http' ? 'https://...' : '/path/to/agent'} className="settings-input" />
          </SettingsField>
          <SettingsField label="Capabilities (JSON)">
            <input value={newCaps} onChange={(e) => setNewCaps(e.target.value)} className="settings-input" />
          </SettingsField>
          <div className="flex gap-2 pt-2">
            <button onClick={() => setShowAdd(false)} className="text-[10px] font-mono text-gray-400 hover:text-gray-300">Cancel</button>
            <button onClick={handleAdd} className="text-[10px] font-mono text-neon-green bg-neon-green/10 border border-neon-green/30 px-3 py-1 rounded hover:bg-neon-green/20">Register</button>
          </div>
        </SettingsSection>
      )}

      <SettingsSection title="Built-in Registration">
        <button onClick={() => api.registerBuiltinRuntimes().then(onRefresh)}
          className="text-[10px] font-mono text-gray-400 hover:text-neon-green transition-colors">
          Re-register built-in runtimes (claude, gemini, codex)
        </button>
      </SettingsSection>
    </div>
  );
}

// ── Advanced Tab ──

function AdvancedTab({ settings }: { settings: Record<string, string> }) {
  const [heartbeatInterval, setHeartbeatInterval] = useState(settings['heartbeatIntervalMs'] ?? '30000');
  const [maxFailures, setMaxFailures] = useState(settings['heartbeatMaxFailures'] ?? '5');
  const [mlEnabled, setMlEnabled] = useState(settings['mlEnabled'] !== 'false');

  return (
    <div className="space-y-5">
      <SettingsSection title="Heartbeat">
        <SettingsField label="Pulse Interval (ms)">
          <select value={heartbeatInterval}
            onChange={(e) => { setHeartbeatInterval(e.target.value); saveSetting('heartbeatIntervalMs', e.target.value); }}
            className="settings-select">
            <option value="5000">5s (aggressive)</option>
            <option value="15000">15s (balanced)</option>
            <option value="30000">30s (default)</option>
            <option value="60000">60s (light)</option>
          </select>
        </SettingsField>
        <SettingsField label="Max Consecutive Failures">
          <input type="number" min="1" max="20" value={maxFailures}
            onChange={(e) => { setMaxFailures(e.target.value); saveSetting('heartbeatMaxFailures', e.target.value); }}
            className="settings-input w-20" />
        </SettingsField>
      </SettingsSection>

      <SettingsSection title="Machine Learning">
        <SettingsToggle label="Enable On-Device ML (trains after each orchestration)" checked={mlEnabled}
          onChange={(v) => { setMlEnabled(v); saveSetting('mlEnabled', String(v)); }} />
        <p className="text-[8px] font-mono text-gray-600 mt-1">
          Models: LinearRegression (time estimation), NaiveBayes (categorization), DecisionTree (conflict prediction)
        </p>
      </SettingsSection>

      <SettingsSection title="Data">
        <button onClick={() => { localStorage.removeItem('xroads-settings'); window.location.reload(); }}
          className="text-[10px] font-mono text-red-400 hover:text-red-300 transition-colors">
          Reset All Settings to Defaults
        </button>
      </SettingsSection>
    </div>
  );
}

// ── Shared Components ──

function SettingsSection({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <h3 className="text-[9px] font-bold font-mono text-gray-500 uppercase tracking-wider mb-2">{title}</h3>
      <div className="space-y-2.5">{children}</div>
    </div>
  );
}

function SettingsField({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block">
      <span className="text-[10px] font-mono text-gray-400 block mb-1">{label}</span>
      {children}
    </label>
  );
}

function SettingsToggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className="flex items-center justify-between cursor-pointer group">
      <span className="text-[11px] font-mono text-gray-300 group-hover:text-gray-200">{label}</span>
      <button onClick={() => onChange(!checked)}
        className={`w-8 h-4 rounded-full transition-colors relative ${checked ? 'bg-neon-green/40' : 'bg-gray-700'}`}>
        <span className={`absolute top-0.5 w-3 h-3 rounded-full transition-all ${checked ? 'left-4 bg-neon-green' : 'left-0.5 bg-gray-500'}`} />
      </button>
    </label>
  );
}

function ShortcutRow({ label, shortcut }: { label: string; shortcut: string }) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-[11px] font-mono text-gray-300">{label}</span>
      <span className="text-[10px] font-mono text-gray-500 bg-gray-800 px-2 py-0.5 rounded">{shortcut}</span>
    </div>
  );
}
