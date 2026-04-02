import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  PtyOutputPayload,
  AgentStatusPayload,
  LogEntryPayload,
  GateEventPayload,
  CostUpdatedPayload,
  OrchestrationUpdatePayload,
  BrainProposal,
} from '../models';

// Event names (must match Rust event_bus constants)
export const EVENT_PTY_OUTPUT = 'pty-output';
export const EVENT_AGENT_STATUS = 'agent-status';
export const EVENT_LOG_ENTRY = 'log-entry';
export const EVENT_GATE_CREATED = 'gate-created';
export const EVENT_GATE_RESOLVED = 'gate-resolved';
export const EVENT_COST_UPDATED = 'cost-updated';
export const EVENT_HEALTH_ALERT = 'health-alert';
export const EVENT_ORCHESTRATION = 'orchestration-update';
export const EVENT_BRAIN_PROPOSAL = 'brain-proposal';
export const EVENT_PREVIEW_URL = 'preview-url';
export const EVENT_AGENT_SCREENSHOT = 'agent-screenshot';

// Typed listener helpers

export function onPtyOutput(callback: (payload: PtyOutputPayload) => void): Promise<UnlistenFn> {
  return listen<PtyOutputPayload>(EVENT_PTY_OUTPUT, (event) => callback(event.payload));
}

export function onAgentStatus(callback: (payload: AgentStatusPayload) => void): Promise<UnlistenFn> {
  return listen<AgentStatusPayload>(EVENT_AGENT_STATUS, (event) => callback(event.payload));
}

export function onLogEntry(callback: (payload: LogEntryPayload) => void): Promise<UnlistenFn> {
  return listen<LogEntryPayload>(EVENT_LOG_ENTRY, (event) => callback(event.payload));
}

export function onGateCreated(callback: (payload: GateEventPayload) => void): Promise<UnlistenFn> {
  return listen<GateEventPayload>(EVENT_GATE_CREATED, (event) => callback(event.payload));
}

export function onGateResolved(callback: (payload: GateEventPayload) => void): Promise<UnlistenFn> {
  return listen<GateEventPayload>(EVENT_GATE_RESOLVED, (event) => callback(event.payload));
}

export function onCostUpdated(callback: (payload: CostUpdatedPayload) => void): Promise<UnlistenFn> {
  return listen<CostUpdatedPayload>(EVENT_COST_UPDATED, (event) => callback(event.payload));
}

export function onOrchestrationUpdate(callback: (payload: OrchestrationUpdatePayload) => void): Promise<UnlistenFn> {
  return listen<OrchestrationUpdatePayload>(EVENT_ORCHESTRATION, (event) => callback(event.payload));
}

export function onBrainProposal(callback: (proposal: BrainProposal) => void): Promise<UnlistenFn> {
  return listen<BrainProposal>(EVENT_BRAIN_PROPOSAL, (event) => callback(event.payload));
}

export function onPreviewURL(callback: (payload: { url: string }) => void): Promise<UnlistenFn> {
  return listen<{ url: string }>(EVENT_PREVIEW_URL, (event) => callback(event.payload));
}

export function onAgentScreenshot(callback: (payload: { slotNumber: number; imageData: string }) => void): Promise<UnlistenFn> {
  return listen<{ slotNumber: number; imageData: string }>(EVENT_AGENT_SCREENSHOT, (event) => callback(event.payload));
}

// Log buffer with FIFO eviction (US-003)

const MAX_LOG_ENTRIES = 5000;
let logBuffer: LogEntryPayload[] = [];
let logListeners: Array<(entries: LogEntryPayload[]) => void> = [];

export function getLogBuffer(): LogEntryPayload[] {
  return logBuffer;
}

export function onLogBufferChange(callback: (entries: LogEntryPayload[]) => void): () => void {
  logListeners.push(callback);
  return () => {
    logListeners = logListeners.filter((l) => l !== callback);
  };
}

export function pushLogEntry(entry: LogEntryPayload): void {
  logBuffer.push(entry);
  if (logBuffer.length > MAX_LOG_ENTRIES) {
    logBuffer = logBuffer.slice(logBuffer.length - MAX_LOG_ENTRIES);
  }
  for (const listener of logListeners) {
    listener(logBuffer);
  }
}

export function clearLogBuffer(): void {
  logBuffer = [];
  for (const listener of logListeners) {
    listener(logBuffer);
  }
}

// Initialize all event subscriptions (call once on app mount)
let initialized = false;
const unlistenFns: UnlistenFn[] = [];

export async function initEventListeners(): Promise<void> {
  if (initialized) return;
  initialized = true;

  // Auto-push log events to buffer
  unlistenFns.push(await onLogEntry((entry) => {
    pushLogEntry(entry);
  }));

  // Auto-route brain proposals to store
  unlistenFns.push(await onBrainProposal((proposal) => {
    import('../stores/appStore').then(({ useAppStore }) => {
      useAppStore.getState().addProposal(proposal);
    });
  }));

  // Auto-route preview URLs to store
  unlistenFns.push(await onPreviewURL((payload) => {
    import('../stores/appStore').then(({ useAppStore }) => {
      useAppStore.getState().setPreviewURL(payload.url);
    });
  }));

  // Auto-route agent screenshots to store
  unlistenFns.push(await onAgentScreenshot((payload) => {
    import('../stores/appStore').then(({ useAppStore }) => {
      useAppStore.getState().setAgentScreenshot(payload.slotNumber, payload.imageData);
    });
  }));
}

export async function destroyEventListeners(): Promise<void> {
  for (const unlisten of unlistenFns) {
    unlisten();
  }
  unlistenFns.length = 0;
  initialized = false;
}
