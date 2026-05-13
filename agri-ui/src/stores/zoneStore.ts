import { create } from 'zustand';
import type { Zone, SensorNode, WeatherData } from '../types';
import { zoneApi, nodeApi } from '../services/api';
import { heweatherApi } from '../services/weather';

interface ZoneStore {
  zones: Zone[];
  currentZone: Zone | null;
  nodes: SensorNode[];
  loading: boolean;
  error: string | null;
  fetchZones: () => Promise<void>;
  fetchZone: (id: string) => Promise<void>;
  createZone: (data: Partial<Zone>) => Promise<Zone>;
  updateZone: (id: string, data: Partial<Zone>) => Promise<void>;
  deleteZone: (id: string) => Promise<void>;
  fetchNodes: (zoneId?: string) => Promise<void>;
}

export const useZoneStore = create<ZoneStore>((set) => ({
  zones: [],
  currentZone: null,
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
}));

interface WeatherStore {
  current: WeatherData | null;
  loading: boolean;
  error: string | null;
  fetchWeather: (location?: string) => Promise<void>;
}

export const useWeatherStore = create<WeatherStore>((set) => ({
  current: null,
  loading: false,
  error: null,

  fetchWeather: async (location?: string) => {
    set({ loading: true, error: null });
    try {
      const weather = await heweatherApi.getWeather(location || '101010100');
      set({ current: weather, loading: false });
    } catch {
      set({ error: '获取天气数据失败', loading: false });
    }
  },
}));