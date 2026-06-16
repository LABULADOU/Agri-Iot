import { create } from 'zustand';
import type { Zone, Assessment, Emergency, TodoItem, AIRecommendation, Device } from '../types';
import { zoneApi, deviceApi } from '../services/api';
import { wsService } from '../services/ws';

interface LatestReadings {
  airTemp: number | undefined;
  humidity: number | undefined;
  soilTemp: number | undefined;
  soilMoisture: number | undefined;
  ec: number | undefined;
}

export interface ZoneNodeReading {
  zoneId: string;
  zoneName: string;
  nodeId: string;
  nodeName: string;
  readings: LatestReadings;
  status: string;
}

interface DashboardState {
  zones: Zone[];
  assessments: Record<string, Assessment>;
  emergencies: Emergency[];
  todoItems: TodoItem[];
  recommendations: AIRecommendation[];
  healthScore: number;
  healthTrend: number;
  nodeReadings: ZoneNodeReading[];

  fetchAll: () => Promise<void>;
  fetchAssessments: () => Promise<void>;
  fetchEmergencies: () => Promise<void>;
  dismissEmergency: (id: string) => void;
  executeRecommendation: (item: TodoItem) => Promise<void>;
  setHealthScore: (score: number) => void;
  stopRealtimeUpdates: () => void;
  _wsUnsub: (() => void) | null;
}

function calcHealthScore(assessments: Record<string, Assessment>): number {
  const scores = Object.values(assessments).map(a => a.score);
  if (scores.length === 0) return 85;
  return Math.round(scores.reduce((a, b) => a + b, 0) / scores.length);
}

function buildTodoItems(zones: Zone[], assessments: Record<string, Assessment>): TodoItem[] {
  const items: TodoItem[] = [];
  for (const zone of zones) {
    const a = assessments[zone.id];
    if (!a) continue;
    if (a.status === 'danger') {
      items.push({
        id: `todo-${zone.id}-danger`,
        zoneId: zone.id,
        zoneName: zone.name,
        type: 'warning',
        message: a.summary,
        timestamp: new Date().toISOString(),
        actionable: true,
      });
    }
  }
  return items;
}

export const useDashboardStore = create<DashboardState>((set, get) => ({
  zones: [],
  assessments: {},
  emergencies: [],
  todoItems: [],
  recommendations: [],
  healthScore: 85,
  healthTrend: 0,
  nodeReadings: [],
  _wsUnsub: null,

  fetchAll: async () => {
    try {
      const [zones, readingsData, devices] = await Promise.all([
        zoneApi.list(),
        fetch('/api/v1/dashboard/node-readings').then(r => r.json()) as Promise<{ areas?: Array<{ area_id: string; area_name: string; nodes: Array<{ node_id: string; status: string; updated_at: number; latest: Record<string, { value: number; unit: string }> }> }> }>,
        deviceApi.list(),
      ]);
      const deviceNameMap = new Map(devices.map(d => [d.node_id, d.name]));

      const nodeReadings: ZoneNodeReading[] = [];
      for (const area of readingsData.areas || []) {
        for (const node of area.nodes) {
          const readings: LatestReadings = { airTemp: undefined, humidity: undefined, soilTemp: undefined, soilMoisture: undefined, ec: undefined };
          const latest = node.latest || {};
          if (latest.temperature?.value !== undefined) readings.airTemp = latest.temperature.value;
          if (latest.humidity?.value !== undefined) readings.humidity = latest.humidity.value;
          if (latest.soil_temperature?.value !== undefined) readings.soilTemp = latest.soil_temperature.value;
          if (latest.soil_moisture?.value !== undefined) readings.soilMoisture = latest.soil_moisture.value;
          if (latest.ec?.value !== undefined) readings.ec = latest.ec.value;
          nodeReadings.push({
            zoneId: area.area_id,
            zoneName: area.area_name,
            nodeId: node.node_id,
            nodeName: deviceNameMap.get(node.node_id) || node.node_id,
            readings,
            status: node.status || 'offline',
          });
        }
      }

      set({ zones, nodeReadings });
      get().fetchAssessments();

      if (!get()._wsUnsub) {
        const unsubTelemetry = wsService.subscribe('telemetry', [], (data) => {
          const msg = data as Record<string, unknown>;
          const nodeId = msg.node_id as string;
          const readings = msg.readings as Array<{ metric: string; value: number }> | undefined;
          if (!nodeId || !readings) return;

          set(state => {
            let newNodeReadings = false;
            const updated = state.nodeReadings.map(nr => {
              if (nr.nodeId !== nodeId) return nr;
              const newReadings = { ...nr.readings };
              for (const r of readings) {
                switch (r.metric) {
                  case 'temperature': newReadings.airTemp = r.value; break;
                  case 'humidity': newReadings.humidity = r.value; break;
                  case 'soil_temperature': newReadings.soilTemp = r.value; break;
                  case 'soil_moisture': newReadings.soilMoisture = r.value; break;
                  case 'ec': newReadings.ec = r.value; break;
                }
              }
              return { ...nr, readings: newReadings, status: 'online' };
            });
            if (!state.nodeReadings.some(nr => nr.nodeId === nodeId)) {
              const device = devices.find(d => d.node_id === nodeId);
              if (device) {
                const newReadings: LatestReadings = { airTemp: undefined, humidity: undefined, soilTemp: undefined, soilMoisture: undefined, ec: undefined };
                for (const r of readings) {
                  switch (r.metric) {
                    case 'temperature': newReadings.airTemp = r.value; break;
                    case 'humidity': newReadings.humidity = r.value; break;
                    case 'soil_temperature': newReadings.soilTemp = r.value; break;
                    case 'soil_moisture': newReadings.soilMoisture = r.value; break;
                    case 'ec': newReadings.ec = r.value; break;
                  }
                }
                updated.push({
                  zoneId: device.area_id || '__unassigned__',
                  zoneName: '未分配',
                  nodeId: device.node_id,
                  nodeName: device.name || device.node_id,
                  readings: newReadings,
                  status: 'online',
                });
                newNodeReadings = true;
              }
            }
            return { nodeReadings: updated };
          });
        });

        const unsubStatus = wsService.subscribe('status_change', [], (data) => {
          const msg = data as Record<string, unknown>;
          const nodeId = msg.node_id as string;
          const status = msg.status as string;
          if (!nodeId || !status) return;

          set(state => ({
            nodeReadings: state.nodeReadings.map(nr =>
              nr.nodeId === nodeId ? { ...nr, status } : nr
            ),
          }));
        });

        set({ _wsUnsub: () => { unsubTelemetry(); unsubStatus(); } });
      }
    } catch (e) {
      console.error('Dashboard fetchAll failed:', e);
    }
  },

  fetchAssessments: async () => {
    const { zones } = get();
    const assessments: Record<string, Assessment> = {};
    for (const zone of zones) {
      try {
        const res = await fetch('/api/v1/ai/assess', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ area_id: zone.id }),
        });
        if (!res.ok) continue;
        const raw = await res.json() as Record<string, unknown>;
        const scores = raw.scores as Record<string, number> || {};
        const overall = scores.overall ?? 85;
        assessments[zone.id] = {
          score: overall,
          status: overall >= 80 ? 'normal' : overall >= 60 ? 'warning' : 'danger',
          summary: raw.deviations ? `存在 ${(raw.deviations as unknown[]).length} 项偏离` : '各项指标正常',
          details: (raw.deviations as Array<{ param: string; current: number; optimal: number }> || [])
            .map(d => `${d.param}: 当前 ${d.current}, 最优 ${d.optimal}`),
        };
      } catch {
        // skip failed assessments
      }
    }
    const healthScore = calcHealthScore(assessments);
    const prevScore = get().healthScore;
    const healthTrend = prevScore === 85 ? 0 : healthScore - prevScore;
    const todoItems = buildTodoItems(zones, assessments);
    set({ assessments, healthScore, healthTrend, todoItems });
  },

  fetchEmergencies: async () => {
    try {
      const res = await fetch('/api/v1/ai/emergency/status');
      const data = await res.json() as { active_emergencies?: Emergency[] };
      set({ emergencies: data.active_emergencies || [] });
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

  stopRealtimeUpdates: () => {
    const unsub = get()._wsUnsub;
    if (unsub) {
      unsub();
      set({ _wsUnsub: null });
    }
  },
}));
