import { create } from 'zustand';
import type { CockpitSession, AgentSlot, UsageSummary } from '../models';

interface AppState {
  // Project
  projectPath: string | null;
  isGitRepo: boolean;
  setProjectPath: (path: string) => void;

  // Session
  session: CockpitSession | null;
  setSession: (session: CockpitSession | null) => void;

  // Slots
  slots: AgentSlot[];
  setSlots: (slots: AgentSlot[]) => void;

  // Cockpit
  showCockpit: boolean;
  toggleCockpit: () => void;
  sessionCost: UsageSummary;
  setSessionCost: (cost: UsageSummary) => void;
  slotCosts: Record<string, UsageSummary>;
  setSlotCosts: (costs: Record<string, UsageSummary>) => void;

  // UI
  showInspector: boolean;
  toggleInspector: () => void;
  showChat: boolean;
  toggleChat: () => void;

  // Single terminal mode (PRD-23)
  expandedSlotId: string | null;
  setExpandedSlotId: (id: string | null) => void;
  toggleExpandedSlot: (id: string) => void;

  // Suite system
  activeSuiteId: string;
  setActiveSuiteId: (id: string) => void;
}

export const useAppStore = create<AppState>((set) => ({
  projectPath: null,
  isGitRepo: false,
  setProjectPath: (path) => {
    set({ projectPath: path, isGitRepo: true });
    // Auto-bootstrap cockpit on project switch
    import('../services/api').then(async (api) => {
      try {
        // Stop previous cockpit session if running
        await api.stopCockpitSession().catch(() => {});
        // Create fresh session
        const session = await api.createSession(path);
        set({ session });
        // Activate (chairman deliberates + slots assigned)
        await api.cockpitActivate(session.id);
        // Start cockpit brain
        await api.startCockpitSession(path);
      } catch (e) {
        console.warn('Auto-cockpit bootstrap failed:', e);
      }
    });
  },

  session: null,
  setSession: (session) => set({ session }),

  slots: [],
  setSlots: (slots) => set({ slots }),

  showCockpit: false,
  toggleCockpit: () => set((s) => ({ showCockpit: !s.showCockpit })),
  sessionCost: { totalInputTokens: 0, totalOutputTokens: 0, totalCostCents: 0, eventCount: 0 },
  setSessionCost: (cost) => set({ sessionCost: cost }),
  slotCosts: {},
  setSlotCosts: (costs) => set({ slotCosts: costs }),

  showInspector: false,
  toggleInspector: () => set((s) => ({ showInspector: !s.showInspector })),
  showChat: true,
  toggleChat: () => set((s) => ({ showChat: !s.showChat })),

  expandedSlotId: null,
  setExpandedSlotId: (id) => set({ expandedSlotId: id }),
  toggleExpandedSlot: (id) => set((s) => ({ expandedSlotId: s.expandedSlotId === id ? null : id })),

  activeSuiteId: 'developer',
  setActiveSuiteId: (id) => set({ activeSuiteId: id }),
}));
