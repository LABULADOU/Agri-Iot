import { create } from 'zustand';
import type { Zone, SensorNode, Assessment, ControlCase } from '../types';
import { zoneApi, nodeApi } from '../services/api';

interface ZoneStore {
  zones: Zone[];
  currentZone: Zone | null;
  currentAssessment: Assessment | null;
  similarCases: ControlCase[];
  nodes: SensorNode[];
  loading: boolean;
  error: string | null;
  fetchZones: () => Promise<void>;
  fetchZone: (id: string) => Promise<void>;
  createZone: (data: Partial<Zone>) => Promise<Zone>;
  updateZone: (id: string, data: Partial<Zone>) => Promise<void>;
  deleteZone: (id: string) => Promise<void>;
  fetchNodes: (zoneId?: string) => Promise<void>;
  fetchAssessment: (areaId: string) => Promise<void>;
  fetchSimilarCases: (areaId: string) => Promise<void>;
}

export const useZoneStore = create<ZoneStore>((set) => ({
  zones: [],
  currentZone: null,
  currentAssessment: null,
  similarCases: [],
  nodes: [],
  loading: false,
  error: null,

  fetchZones: async () => {
    set({ loading: true, error: null });
    try {
      const zones = await zoneApi.list();
      set({ zones, loading: false });
    } catch {
      set({ error: '获取区域列表失败', loading: false });
    }
  },

  fetchZone: async (id: string) => {
    set({ loading: true, error: null });
    try {
      const zone = await zoneApi.get(id);
      set({ currentZone: zone, loading: false });
    } catch {
      set({ error: '获取区域详情失败', loading: false });
    }
  },

  createZone: async (data: Partial<Zone>) => {
    const zone = await zoneApi.create(data);
    set(state => ({ zones: [...state.zones, zone] }));
    return zone;
  },

  updateZone: async (id: string, data: Partial<Zone>) => {
    const zone = await zoneApi.update(id, data);
    set(state => ({
      zones: state.zones.map(z => z.id === id ? zone : z),
      currentZone: state.currentZone?.id === id ? zone : state.currentZone,
    }));
  },

  deleteZone: async (id: string) => {
    await zoneApi.delete(id);
    set(state => ({
      zones: state.zones.filter(z => z.id !== id),
      currentZone: state.currentZone?.id === id ? null : state.currentZone,
    }));
  },

  fetchNodes: async (zoneId?: string) => {
    set({ loading: true, error: null });
    try {
      const nodes = await nodeApi.list(zoneId);
      set({ nodes, loading: false });
    } catch {
      set({ error: '获取节点列表失败', loading: false });
    }
  },

  fetchAssessment: async (areaId: string) => {
    try {
      const res = await fetch('/api/v1/ai/assess', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ area_id: areaId }),
      });
      const data = await res.json() as Assessment;
      set({ currentAssessment: data });
    } catch {
      set({ currentAssessment: null });
    }
  },

  fetchSimilarCases: async (areaId: string) => {
    try {
      const res = await fetch(`/api/v1/ai/knowledge/cases?area_id=${areaId}`);
      const data = await res.json() as ControlCase[];
      set({ similarCases: data || [] });
    } catch {
      set({ similarCases: [] });
    }
  },
}));
