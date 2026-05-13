import { create } from 'zustand';
import { wsService } from '../services/ws';

interface RealtimeReading {
  nodeId: string;
  metric: string;
  value: number;
  unit: string;
  timestamp: string;
}

interface RealtimeStore {
  readings: Map<string, RealtimeReading[]>;
  lastUpdate: string | null;
  connected: boolean;
  connect: () => void;
  disconnect: () => void;
  getNodeReadings: (nodeId: string) => RealtimeReading[];
  getMetricValue: (nodeId: string, metric: string) => number | null;
}

export const useRealtimeStore = create<RealtimeStore>((set, get) => ({
  readings: new Map(),
  lastUpdate: null,
  connected: false,

  connect: () => {
    wsService.connect();
    set({ connected: true });

    const unsubscribe = wsService.subscribe((data) => {
      set(state => {
        const newReadings = new Map(state.readings);
        const nodeReadings = data.readings.map(r => ({
          nodeId: data.nodeId,
          ...r,
        }));

        const existing = newReadings.get(data.nodeId) || [];
        const updated = [...existing, ...nodeReadings].slice(-100);
        newReadings.set(data.nodeId, updated);

        return {
          readings: newReadings,
          lastUpdate: new Date().toISOString(),
        };
      });
    });

    return unsubscribe;
  },

  disconnect: () => {
    wsService.disconnect();
    set({ connected: false, readings: new Map() });
  },

  getNodeReadings: (nodeId: string) => {
    return get().readings.get(nodeId) || [];
  },

  getMetricValue: (nodeId: string, metric: string) => {
    const readings = get().readings.get(nodeId) || [];
    const latest = readings.filter(r => r.metric === metric).pop();
    return latest?.value ?? null;
  },
}));