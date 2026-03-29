// XRoads Tauri — TypeScript model interfaces
// Mirrors Rust structs in src-tauri/src/models/

// Enums
export type CockpitSessionStatus = 'idle' | 'initializing' | 'active' | 'paused' | 'closed';
export type AgentSlotStatus = 'empty' | 'provisioning' | 'running' | 'waiting_approval' | 'paused' | 'done' | 'error';
export type ExecutionGateStatus = 'pending' | 'dry_run' | 'awaiting_approval' | 'executing' | 'completed' | 'rejected' | 'rolled_back';
export type RiskLevel = 'low' | 'medium' | 'high' | 'critical';
export type MessageType = 'status' | 'log' | 'error' | 'blocker' | 'chairman_brief' | 'handoff';
export type AgentType = 'claude' | 'gemini' | 'codex';

// Entities
export interface CockpitSession {
  id: string;
  projectPath: string;
  status: CockpitSessionStatus;
  chairmanBrief?: string;
  createdAt: string;
  updatedAt: string;
}

export interface AgentSlot {
  id: string;
  cockpitSessionId: string;
  slotIndex: number;
  status: AgentSlotStatus;
  agentType: string;
  worktreePath?: string;
  branchName?: string;
  skillId?: string;
  currentTask?: string;
  createdAt: string;
  updatedAt: string;
}

export interface MetierSkill {
  id: string;
  name: string;
  family: string;
  skillMdPath: string;
  requiredMcps?: string;
  description?: string;
  createdAt: string;
}

export interface AgentMessage {
  id: string;
  content: string;
  messageType: MessageType;
  fromSlotId: string;
  toSlotId?: string;
  isBroadcast: boolean;
  readAt?: string;
  createdAt: string;
}

export interface ExecutionGate {
  id: string;
  agentSlotId: string;
  status: ExecutionGateStatus;
  operationType: string;
  operationPayload: string;
  riskLevel: RiskLevel;
  estimatedImpact?: string;
  approvedBy?: string;
  approvedAt?: string;
  deniedReason?: string;
  rollbackPayload?: string;
  auditEntry?: string;
  createdAt: string;
  updatedAt: string;
}

export interface CostEvent {
  id: string;
  agentSlotId: string;
  provider: string;
  model: string;
  inputTokens: number;
  outputTokens: number;
  costCents: number;
  createdAt: string;
}

export interface UsageSummary {
  totalInputTokens: number;
  totalOutputTokens: number;
  totalCostCents: number;
  eventCount: number;
}

export interface Worktree {
  id: string;
  path: string;
  branch: string;
  agentId?: string;
  status: string;
  createdAt: string;
}

export interface OrchestrationRecord {
  id: string;
  startedAt: string;
  finishedAt?: string;
  prdName: string;
  resultSummary: string;
  mergedBranches: string[];
  conflicts: string[];
  totalStories: number;
  completedStories: number;
  totalCostCents: number;
}
