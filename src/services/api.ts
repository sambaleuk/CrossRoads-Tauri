import { invoke } from '@tauri-apps/api/core';
import type {
  CockpitSession, AgentSlot, CostEvent, UsageSummary,
  ExecutionGate, AgentMessage, MetierSkill
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
