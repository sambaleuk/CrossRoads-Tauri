import { invoke } from '@tauri-apps/api/core';
import type {
  CockpitSession, AgentSlot, CostEvent, UsageSummary,
  ExecutionGate, AgentMessage, MetierSkill,
  SpawnRequest, AgentHealth, AgentMetrics,
  ParsedPrd, ExecutionLayer, LayerDispatchPlan,
  OrchestrationStart, OrchestrationRecordDb, McpSession,
  ChairmanInput, ChairmanOutput, DangerousOperation, PolicyDecisionStr
} from '../models';

// Session
export const createSession = (projectPath: string) =>
  invoke<CockpitSession>('create_session', { projectPath });

export const fetchSession = (id: string) =>
  invoke<CockpitSession | null>('fetch_session', { id });

export const updateSession = (id: string, status: string, chairmanBrief?: string) =>
  invoke<void>('update_session', { id, status, chairmanBrief });

export const deleteSession = (id: string) =>
  invoke<void>('delete_session', { id });

export const activeSession = (projectPath: string) =>
  invoke<CockpitSession | null>('active_session', { projectPath });

export const fetchAllSessions = () =>
  invoke<CockpitSession[]>('fetch_all_sessions');

// Slots
export const createSlot = (sessionId: string, slotIndex: number, agentType: string, skillId?: string, branchName?: string) =>
  invoke<AgentSlot>('create_slot', { sessionId, slotIndex, agentType, skillId, branchName });

export const updateSlot = (id: string, status: string, currentTask?: string) =>
  invoke<void>('update_slot', { id, status, currentTask });

export const fetchSlots = (sessionId: string) =>
  invoke<AgentSlot[]>('fetch_slots', { sessionId });

// Cost tracking
export const recordUsage = (slotId: string, provider: string, model: string, inputTokens: number, outputTokens: number) =>
  invoke<CostEvent>('record_usage', { slotId, provider, model, inputTokens, outputTokens });

export const costSummarySlot = (slotId: string) =>
  invoke<UsageSummary>('cost_summary_slot', { slotId });

export const costSummarySession = (sessionId: string) =>
  invoke<UsageSummary>('cost_summary_session', { sessionId });

// Gates
export const createGate = (slotId: string, operationType: string, payload: string, riskLevel: string) =>
  invoke<ExecutionGate>('create_gate', { slotId, operationType, payload, riskLevel });

export const approveGate = (id: string, approvedBy: string) =>
  invoke<void>('approve_gate', { id, approvedBy });

export const rejectGate = (id: string, reason: string) =>
  invoke<void>('reject_gate', { id, reason });

export const fetchGates = (slotId: string) =>
  invoke<ExecutionGate[]>('fetch_gates', { slotId });

// Messages
export const publishMessage = (content: string, messageType: string, fromSlotId: string, toSlotId?: string, isBroadcast = false) =>
  invoke<AgentMessage>('publish_message', { content, messageType, fromSlotId, toSlotId, isBroadcast });

export const fetchMessages = (slotId: string) =>
  invoke<AgentMessage[]>('fetch_messages', { slotId });

// Skills
export const createSkillRecord = (name: string, family: string, skillMdPath: string, description?: string) =>
  invoke<MetierSkill>('create_skill', { name, family, skillMdPath, description });

export const findSkill = (name: string) =>
  invoke<MetierSkill | null>('find_skill', { name });

// Git
export const isGitRepo = (path: string) =>
  invoke<boolean>('is_git_repo', { path });

export const gitCurrentBranch = (path: string) =>
  invoke<string>('git_current_branch', { path });

export const gitRecentCommits = (path: string, count: number) =>
  invoke<Array<{ shortSha: string; message: string; author: string; date: string }>>('git_recent_commits', { path, count });

export const gitBranches = (path: string) =>
  invoke<string[]>('git_branches', { path });

export const gitCreateWorktree = (repoPath: string, worktreePath: string, branch: string) =>
  invoke<void>('git_create_worktree', { repoPath, worktreePath, branch });

export const gitCoordinateMerge = (repoPath: string, branches: string[]) =>
  invoke<{ success: boolean; mergedBranches: string[]; conflicts: string[]; rolledBack: boolean }>('git_coordinate_merge', { repoPath, branches });

// CLI
export const detectCliTools = () =>
  invoke<Array<{ name: string; available: boolean; version?: string; path?: string }>>('detect_cli_tools');

// Agent Lifecycle (PRD-14)
export const spawnAgent = (req: SpawnRequest) =>
  invoke<string>('spawn_agent', { req });

export const abortAgent = (slotId: string) =>
  invoke<void>('abort_agent', { slotId });

export const agentHealth = (slotId: string) =>
  invoke<AgentHealth | null>('agent_health', { slotId });

export const allAgentHealth = () =>
  invoke<AgentHealth[]>('all_agent_health');

export const failoverAgent = (slotId: string) =>
  invoke<string>('failover_agent', { slotId });

export const handleAlertAction = (slotId: string, action: string) =>
  invoke<void>('handle_alert_action', { slotId, action });

export const checkAgentsHealth = () =>
  invoke<void>('check_agents_health');

// Metrics (PRD-14)
export const fetchAgentMetrics = (slotId: string) =>
  invoke<AgentMetrics>('fetch_agent_metrics', { slotId });

export const recordStoryCompleted = (slotId: string, storyTimeMs: number) =>
  invoke<void>('record_story_completed', { slotId, storyTimeMs });

export const recordStoryFailed = (slotId: string) =>
  invoke<void>('record_story_failed', { slotId });

// Orchestration Engine (PRD-15)
export const parsePrd = (path: string) =>
  invoke<ParsedPrd>('parse_prd', { path });

export const detectPrdFiles = (dir: string) =>
  invoke<string[]>('detect_prd_files', { dir });

export const buildExecutionLayers = (path: string) =>
  invoke<ExecutionLayer[]>('build_execution_layers', { path });

export const createDispatchPlans = (path: string, numSlots: number) =>
  invoke<LayerDispatchPlan[]>('create_dispatch_plans', { path, numSlots });

export const startOrchestration = (sessionId: string, prdPath: string) =>
  invoke<OrchestrationStart>('start_orchestration', { sessionId, prdPath });

export const updateOrchestrationProgress = (recordId: string, completed: number, failed: number, currentLayer: number) =>
  invoke<void>('update_orchestration_progress', { recordId, completed, failed, currentLayer });

export const completeOrchestration = (recordId: string, summary: string, mergedBranches: string[], conflicts: string[], totalCost: number) =>
  invoke<void>('complete_orchestration', { recordId, summary, mergedBranches, conflicts, totalCost });

export const fetchOrchestrationRecord = (recordId: string) =>
  invoke<OrchestrationRecordDb | null>('fetch_orchestration_record', { recordId });

export const fetchOrchestrationRecords = (sessionId: string) =>
  invoke<OrchestrationRecordDb[]>('fetch_orchestration_records', { sessionId });

// MCP (PRD-16)
export const mcpDetectNode = () =>
  invoke<string>('mcp_detect_node');

export const mcpFindServer = (projectRoot?: string) =>
  invoke<string>('mcp_find_server', { projectRoot });

export const mcpPersistSession = (worktreePath: string, sessionId: string, projectPath: string, agentType: string) =>
  invoke<void>('mcp_persist_session', { worktreePath, sessionId, projectPath, agentType });

export const mcpLoadSession = (worktreePath: string, sessionId: string) =>
  invoke<McpSession | null>('mcp_load_session', { worktreePath, sessionId });

export const mcpRecordDecision = (worktreePath: string, sessionId: string, decisionType: string, description: string, context?: string) =>
  invoke<void>('mcp_record_decision', { worktreePath, sessionId, decisionType, description, context });

export const mcpGenerateHandoff = (worktreePath: string, sessionId: string, maxTokens?: number) =>
  invoke<string>('mcp_generate_handoff', { worktreePath, sessionId, maxTokens });

// Event Bus (PRD-17)
export const emitAgentStatus = (slotId: string, status: string, progress?: number, task?: string, agentType?: string) =>
  invoke<void>('emit_agent_status', { slotId, status, progress, task, agentType });

export const emitLogEntry = (level: string, source: string, message: string, slotId?: string) =>
  invoke<void>('emit_log_entry', { level, source, message, slotId });

export const emitGateEvent = (gateId: string, slotId: string, operationType: string, riskLevel: string, status: string) =>
  invoke<void>('emit_gate_event', { gateId, slotId, operationType, riskLevel, status });

export const flushPtyBuffers = () =>
  invoke<void>('flush_pty_buffers');

// Cockpit Logic (PRD-18)
export const cockpitActivate = (sessionId: string) =>
  invoke<ChairmanOutput>('cockpit_activate', { sessionId });

export const cockpitPause = (sessionId: string) =>
  invoke<void>('cockpit_pause', { sessionId });

export const cockpitResume = (sessionId: string) =>
  invoke<void>('cockpit_resume', { sessionId });

export const cockpitClose = (sessionId: string) =>
  invoke<void>('cockpit_close', { sessionId });

export const cockpitReadContext = (projectPath: string) =>
  invoke<ChairmanInput>('cockpit_read_context', { projectPath });

export const cockpitDeliberate = (projectPath: string) =>
  invoke<ChairmanOutput>('cockpit_deliberate', { projectPath });

// SafeExecutor (PRD-19)
export const detectDangerousOps = (text: string) =>
  invoke<DangerousOperation[]>('detect_dangerous_ops', { text });

export const safeTriggerGate = (slotId: string, pattern: string, matchedText: string, riskLevel: string, description: string) =>
  invoke<string>('safe_trigger_gate', { slotId, pattern, matchedText, riskLevel, description });

export const safeApproveGate = (gateId: string, slotId: string, approvedBy: string) =>
  invoke<void>('safe_approve_gate', { gateId, slotId, approvedBy });

export const safeRejectGate = (gateId: string, slotId: string, reason: string) =>
  invoke<void>('safe_reject_gate', { gateId, slotId, reason });

export const evaluatePolicy = (riskLevel: string, pattern: string) =>
  invoke<PolicyDecisionStr>('evaluate_policy', { riskLevel, pattern });
