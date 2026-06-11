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
  _unsub: (() => void) | null;
  connect: () => void;
  disconnect: () => void;
  getNodeReadings: (nodeId: string) => RealtimeReading[];
  getMetricValue: (nodeId: string, metric: string) => number | null;
}

export const useRealtimeStore = create<RealtimeStore>((set, get) => ({
  readings: new Map(),
  lastUpdate: null,
  connected: false,
  _unsub: null as (() => void) | null,

  connect: () => {
    wsService.connect();
    set({ connected: true });

    const unsub = wsService.subscribe('telemetry', [], (data) => {
      set(state => {
        const newReadings = new Map(state.readings);
        const msg = data as Record<string, unknown>;
        const nodeId = msg.node_id as string || msg.nodeId as string || '';
        const readings = (msg.readings as Array<Record<string, unknown>> || [msg])
          .map(r => ({
            nodeId: r.node_id as string || nodeId,
            metric: r.metric as string || '',
            value: Number(r.value) || 0,
            unit: r.unit as string || '',
            timestamp: r.timestamp as string || new Date().toISOString(),
          }));

        if (!readings.length || !readings[0].metric) return state;

        const existing = newReadings.get(nodeId) || [];
        const updated = [...existing, ...readings].slice(-100);
        newReadings.set(nodeId, updated);

        return {
          readings: newReadings,
          lastUpdate: new Date().toISOString(),
        };
      });
    });

    set({ _unsub: unsub });
  },

  disconnect: () => {
    const unsub = get()._unsub;
    if (unsub) unsub();
    wsService.disconnect();
    set({ connected: false, readings: new Map(), _unsub: null });
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
