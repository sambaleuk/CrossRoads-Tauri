import { create } from 'zustand';
import type { CockpitSession, AgentSlot, UsageSummary, BrainProposal } from '../models';

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

  // Review Ribbon + Brain Proposals
  showReviewRibbon: boolean;
  toggleReviewRibbon: () => void;
  setShowReviewRibbon: (show: boolean) => void;
  pendingProposals: Record<string, BrainProposal>;
  addProposal: (proposal: BrainProposal) => void;
  removeProposal: (id: string) => void;
  clearProposals: () => void;

  // Preview + Agent Vision
  previewURL: string;
  setPreviewURL: (url: string) => void;
  agentScreenshots: Record<number, string>; // slot → base64 PNG
  setAgentScreenshot: (slot: number, data: string) => void;
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
        // Activate (chairman deliberates + slots assigned + agents launched)
        await api.cockpitActivate(session.id);
        // Fetch created slots from DB and update store
        let slots = await api.fetchSlots(session.id);
        set({ slots, showCockpit: true });
        // Start cockpit brain
        await api.startCockpitSession(path).catch(() => {});
        // Re-fetch slots after a short delay (agents may have changed status)
        await new Promise(r => setTimeout(r, 2000));
        slots = await api.fetchSlots(session.id);
        set({ slots });
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

  showReviewRibbon: false,
  toggleReviewRibbon: () => set((s) => ({ showReviewRibbon: !s.showReviewRibbon })),
  setShowReviewRibbon: (show) => set({ showReviewRibbon: show }),
  pendingProposals: {},
  addProposal: (proposal) => set((s) => ({
    pendingProposals: { ...s.pendingProposals, [proposal.id]: proposal },
    showReviewRibbon: true, // Auto-show ribbon when proposal arrives
  })),
  removeProposal: (id) => set((s) => {
    const { [id]: _, ...rest } = s.pendingProposals;
    return {
      pendingProposals: rest,
      showReviewRibbon: Object.keys(rest).length > 0 ? s.showReviewRibbon : false,
    };
  }),
  clearProposals: () => set({ pendingProposals: {}, showReviewRibbon: false }),

  previewURL: '',
  setPreviewURL: (url) => set({ previewURL: url }),
  agentScreenshots: {},
  setAgentScreenshot: (slot, data) => set((s) => ({
    agentScreenshots: { ...s.agentScreenshots, [slot]: data },
  })),
}));
