import { create } from 'zustand';
import type { Zone } from '../types';

interface Assessment {
  score: number;
  status: 'normal' | 'warning' | 'danger';
  summary: string;
}

interface Emergency {
  id: string;
  type: string;
  message: string;
  severity: 'high' | 'critical';
  timestamp: string;
}

export interface TodoItem {
  id: string;
  zoneId?: string;
  zoneName: string;
  type: 'warning' | 'attention' | 'offline';
  message: string;
  aiRecommendation?: string;
  timestamp: string;
  actionable: boolean;
}

interface AIRecommendation {
  id: string;
  content: string;
  targetArea: string;
  caseLink?: string;
}

interface DashboardState {
  zones: Zone[];
  assessments: Record<string, Assessment>;
  emergencies: Emergency[];
  todoItems: TodoItem[];
  recommendations: AIRecommendation[];
  healthScore: number;

  fetchAll: () => Promise<void>;
  fetchEmergencies: () => Promise<void>;
  dismissEmergency: (id: string) => void;
  executeRecommendation: (item: TodoItem) => Promise<void>;
  setHealthScore: (score: number) => void;
}

export const useDashboardStore = create<DashboardState>((set) => ({
  zones: [],
  assessments: {},
  emergencies: [],
  todoItems: [],
  recommendations: [],
  healthScore: 85,

  fetchAll: async () => {
    try {
      const res = await fetch('/api/v1/areas');
      const zones = await res.json() as Zone[];
      set({ zones });
    } catch {
      // API not available - keep defaults
    }
  },

  fetchEmergencies: async () => {
    try {
      const res = await fetch('/api/v1/ai/emergency/status');
      const data = await res.json() as { emergencies: Emergency[] };
      set({ emergencies: data.emergencies || [] });
    } catch {
      // keep current
    }
  },

  dismissEmergency: (id: string) => {
    set(state => ({
      emergencies: state.emergencies.filter(e => e.id !== id),
    }));
  },

  executeRecommendation: async (item: TodoItem) => {
    console.log('Executing:', item.id, item.aiRecommendation);
  },

  setHealthScore: (score: number) => {
    set({ healthScore: score });
  },
}));
