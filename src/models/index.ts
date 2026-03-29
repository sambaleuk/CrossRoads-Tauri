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

// PRD-14: Agent Lifecycle types

export interface SpawnRequest {
  slotId: string;
  sessionId: string;
  slotIndex: number;
  agentType: AgentType;
  worktreePath: string;
  prdPath: string;
  branchName: string;
  maxIterations?: number;
  sleepSeconds?: number;
  skillContent?: string;
  handoffContext?: string;
}

export interface AgentHealth {
  slotId: string;
  processId?: string;
  agentType: string;
  status: string;
  progressPct?: number;
  currentTask?: string;
  lastOutputAt?: string;
  failoverAttempts: number;
}

export interface HealthAlert {
  slotId: string;
  alertType: string;
  message: string;
  actions: string[];
  createdAt: string;
}

export interface AgentMetrics {
  id: string;
  agentSlotId: string;
  totalStoriesCompleted: number;
  totalStoriesFailed: number;
  avgStoryTimeMs: number;
  conflictsEncountered: number;
  failoverAttempts: number;
  lastStoryStartedAt?: string;
  updatedAt: string;
}

// PRD-15: Orchestration Engine types

export interface ParsedStory {
  id: string;
  title: string;
  description: string;
  status: string;
  tests: string[];
  dependencies: string[];
  priority: string;
}

export interface ParsedPrd {
  featureName: string;
  description: string;
  branch?: string;
  status: string;
  stories: ParsedStory[];
}

export interface ExecutionLayer {
  index: number;
  storyIds: string[];
}

export interface DispatchAssignment {
  storyId: string;
  slotIndex: number;
  status: string;
}

export interface LayerDispatchPlan {
  layerIndex: number;
  assignments: DispatchAssignment[];
}

export interface OrchestrationStart {
  recordId: string;
  featureName: string;
  totalStories: number;
  totalLayers: number;
  resumeLayer: number;
  layers: ExecutionLayer[];
  plans: LayerDispatchPlan[];
}

// PRD-16: MCP types

export interface McpSession {
  sessionId: string;
  projectPath: string;
  agentType: string;
  startedAt: string;
  decisions: McpDecision[];
}

export interface McpDecision {
  decisionType: string;
  description: string;
  context?: string;
  timestamp: string;
}

// PRD-17: Event Bus payload types

export interface PtyOutputPayload {
  slotId: string;
  text: string;
}

export interface AgentStatusPayload {
  slotId: string;
  status: AgentSlotStatus;
  progress?: number;
  task?: string;
  agentType?: string;
}

export interface LogEntryPayload {
  level: string;
  source: string;
  message: string;
  timestamp: string;
  slotId?: string;
}

export interface GateEventPayload {
  gateId: string;
  slotId: string;
  operationType: string;
  riskLevel: string;
  status: string;
}

export interface CostUpdatedPayload {
  slotId: string;
  provider: string;
  model: string;
  inputTokens: number;
  outputTokens: number;
  costCents: number;
  totalCostCents: number;
}

export interface OrchestrationUpdatePayload {
  recordId: string;
  eventType: string;
  currentLayer?: number;
  completedStories?: number;
  totalStories?: number;
  message?: string;
}

// PRD-18: Cockpit Logic types

export interface ChairmanInput {
  projectPath: string;
  currentBranch: string;
  recentCommits: Array<{ shortSha: string; message: string; author: string }>;
  branches: string[];
  prdSummary?: string;
  previousSession?: { sessionId: string; status: string; slotCount: number };
}

export interface SlotAssignment {
  slotIndex: number;
  agentType: string;
  skillName: string;
  branchName: string;
  taskDescription: string;
}

export interface ChairmanOutput {
  brief: string;
  assignments: SlotAssignment[];
}

// PRD-19: SafeExecutor types

export interface DangerousOperation {
  pattern: string;
  matchedText: string;
  riskLevel: string;
  description: string;
}

export type PolicyDecisionStr = 'auto_approve' | 'require_dry_run' | 'require_human_approval';

// PRD-20: Skill System types

export interface LoadedSkill {
  name: string;
  family: string;
  description?: string;
  requiredMcps?: string;
  content: string;
  sourcePath: string;
  isUserOverride: boolean;
}

// PRD-21: Session Persistence types

export interface PersistedSession {
  sessionId: string;
  projectPath: string;
  status: string;
  chairmanBrief?: string;
  slots: Array<{
    slotId: string;
    slotIndex: number;
    status: string;
    agentType: string;
    worktreePath?: string;
    branchName?: string;
    currentTask?: string;
  }>;
  savedAt: string;
}

export interface RecoveryInfo {
  sessionId: string;
  projectPath: string;
  status: string;
  chairmanBrief?: string;
  slotCount: number;
  runningSlots: number;
  errorSlots: number;
  createdAt: string;
}

export interface HistoryEntry {
  prdPath: string;
  prdName: string;
  projectPath: string;
  startedAt: string;
  finishedAt: string;
  totalStories: number;
  completedStories: number;
  failedStories: number;
  totalCostCents: number;
  mergedBranches: string[];
  conflicts: string[];
  durationSeconds: number;
}

export interface OrphanedWorktree {
  path: string;
  branch: string;
  reason: string;
}

export interface OrchestrationRecordDb {
  id: string;
  cockpitSessionId: string;
  prdPath: string;
  prdName: string;
  status: string;
  totalStories: number;
  completedStories: number;
  failedStories: number;
  currentLayer: number;
  layersJson?: string;
  resultSummary?: string;
  mergedBranches?: string;
  conflicts?: string;
  totalCostCents: number;
  startedAt: string;
  finishedAt?: string;
  updatedAt: string;
}

// Phase 5: Org Chart
export interface OrgRole {
  id: string;
  sessionId: string;
  name: string;
  roleType: string;
  parentRoleId?: string;
  assignedSlotId?: string;
  goalDescription?: string;
  skillNames?: string;
  authority: string;
  createdAt: string;
  updatedAt: string;
}

export interface OrgRoleNode extends OrgRole {
  children: OrgRoleNode[];
}

// Phase 5: Budget Control
export interface BudgetConfig {
  id: string;
  sessionId: string;
  slotId?: string;
  budgetCents: number;
  warningThresholdPct: number;
  hardStopEnabled: boolean;
  throttleEnabled: boolean;
  dailyLimitCents?: number;
  perStoryLimitCents?: number;
  createdAt: string;
  updatedAt: string;
}

export interface BudgetAlert {
  id: string;
  budgetConfigId: string;
  alertType: string;
  currentSpendCents: number;
  budgetCents: number;
  percentUsed: number;
  message: string;
  acknowledged: boolean;
  createdAt: string;
}

export interface BudgetStatus {
  status: 'ok' | 'warning' | 'exceeded';
  percentUsed: number;
  remainingCents: number;
  projectedTotal: number;
}

export interface CostProjection {
  currentSpend: number;
  burnRateCentsPerHour: number;
  projectedTotal: number;
  timeToExhaustionMinutes: number;
  overBudget: boolean;
}

// Phase 5: Heartbeat
export interface HeartbeatConfig {
  id: string;
  sessionId: string;
  slotId?: string;
  intervalMs: number;
  enabled: boolean;
  lastPulseAt?: string;
  lastPulseResult?: string;
  consecutiveFailures: number;
  maxFailures: number;
  createdAt: string;
  updatedAt: string;
}

export interface ScheduledRun {
  id: string;
  projectPath: string;
  prdPath: string;
  cronExpression?: string;
  triggerType: string;
  triggerConfig?: string;
  roleTemplateName?: string;
  budgetPreset?: string;
  enabled: boolean;
  lastRunAt?: string;
  lastRunResult?: string;
  nextRunAt?: string;
  createdAt: string;
}

export interface PulseResult {
  alive: boolean;
  gitChanges: number;
  testsPassed: number;
  testsFailed: number;
  storiesCompleted: number;
  mergeable: boolean;
  errors: string[];
}

// Phase 5: Workspace
export interface Workspace {
  id: string;
  name: string;
  projectPath: string;
  color: string;
  icon?: string;
  isActive: boolean;
  maxSlots: number;
  totalBudgetCents?: number;
  metadata?: string;
  lastAccessedAt: string;
  createdAt: string;
}

// Phase 5: Agent Runtime
export interface AgentRuntimeConfig {
  id: string;
  name: string;
  runtimeType: string;
  command?: string;
  url?: string;
  dockerImage?: string;
  healthCheckCommand?: string;
  configSchema?: string;
  configDefaults?: string;
  capabilities: string;
  icon?: string;
  color?: string;
  isBuiltin: boolean;
  isEnabled: boolean;
  createdAt: string;
}

// Phase 5: Config Versioning
export interface ConfigSnapshot {
  id: string;
  sessionId?: string;
  workspaceId?: string;
  configType: string;
  version: number;
  data: string;
  diff?: string;
  changedBy: string;
  changeReason?: string;
  createdAt: string;
}

// Phase 5: Learning
export interface LearningRecord {
  id: string;
  sessionId: string;
  storyId: string;
  storyTitle: string;
  storyComplexity: string;
  agentType: string;
  runtimeId?: string;
  model?: string;
  durationMs: number;
  costCents: number;
  filesChanged: number;
  linesAdded: number;
  linesRemoved: number;
  testsRun: number;
  testsPassed: number;
  testsFailed: number;
  conflictsEncountered: number;
  retriesNeeded: number;
  success: boolean;
  failureReason?: string;
  filePatterns: string;
  createdAt: string;
}

export interface PerformanceProfile {
  id: string;
  agentType: string;
  runtimeId?: string;
  taskCategory: string;
  totalExecutions: number;
  successRate: number;
  avgDurationMs: number;
  avgCostCents: number;
  avgFilesChanged: number;
  avgTestPassRate: number;
  conflictRate: number;
  lastUpdatedAt: string;
}

export interface AgentRecommendation {
  agentType: string;
  confidence: number;
  reason: string;
}

export interface ConflictPrediction {
  storyA: string;
  storyB: string;
  overlappingPatterns: string[];
  risk: string;
}
