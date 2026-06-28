import { create } from 'zustand';
import type { Zone, Assessment, Emergency, TodoItem, Device, AnomalyEvent } from '../types';
import { zoneApi, deviceApi, aiApi } from '../services/api';
import { wsService } from '../services/ws';
import { useRealtimeStore } from './realtimeStore';

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
  anomalyCount: number;
  anomalySeverity?: string;
}

interface DashboardState {
  zones: Zone[];
  assessments: Record<string, Assessment>;
  emergencies: Emergency[];
  todoItems: TodoItem[];
  healthScore: number;
  healthTrend: number;
  nodeReadings: ZoneNodeReading[];
  anomalies: Record<string, AnomalyEvent[]>;

  fetchAll: () => Promise<void>;
  fetchAssessments: () => Promise<void>;
  fetchEmergencies: () => Promise<void>;
  dismissEmergency: (id: string) => void;
  executeRecommendation: (item: TodoItem) => Promise<void>;
  setHealthScore: (score: number) => void;
  stopRealtimeUpdates: () => void;
  _wsUnsub: (() => void) | null;
  _realtimeUnsub: (() => void) | null;
  _statusUnsub: (() => void) | null;
  _anomalyUnsub: (() => void) | null;
  _assessTimer: number | undefined;
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
  healthScore: 85,
  healthTrend: 0,
  nodeReadings: [],
  anomalies: {},
  _wsUnsub: null,
  _realtimeUnsub: null,
  _statusUnsub: null,
  _anomalyUnsub: null,
  _assessTimer: undefined,

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
            anomalyCount: 0,
          });
        }
      }

      set({ zones, nodeReadings });
      get().fetchAssessments();

      if (!get()._realtimeUnsub) {
        const unsubRealtime = useRealtimeStore.getState().onTelemetry((msg) => {
          const nodeId = msg.node_id as string;
          const readings = msg.readings as Array<{ metric: string; value: number }> | undefined;
          if (!nodeId || !readings) return;

          set(state => {
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
                  anomalyCount: 0,
                });
              }
            }
            return { nodeReadings: updated };
          });
          clearTimeout(get()._assessTimer);
          const timer = window.setTimeout(() => get().fetchAssessments(), 10000);
          set({ _assessTimer: timer });
        });
        set({ _realtimeUnsub: unsubRealtime });
      }

      if (!get()._statusUnsub) {
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
        set({ _statusUnsub: unsubStatus });
      }

      if (!get()._anomalyUnsub) {
        const unsubAnomaly = wsService.subscribe('anomaly', [], (data) => {
          const evt = data as Record<string, unknown>;
          const nodeId = evt.node_id as string;
          const severity = evt.severity as string;
          if (!nodeId) return;

          const anomaly: AnomalyEvent = {
            node_id: nodeId,
            metric: evt.metric as string,
            anomaly_type: evt.anomaly_type as AnomalyEvent['anomaly_type'],
            severity: severity as AnomalyEvent['severity'],
            value_original: evt.value_original as number | undefined,
            message: evt.message as string,
            timestamp: evt.timestamp as number,
          };

          set(state => {
            const existing = state.anomalies[nodeId] || [];
            const updated = [anomaly, ...existing].slice(0, 50);
            const anomalyCount = existing.length + 1;
            return {
              anomalies: { ...state.anomalies, [nodeId]: updated },
              nodeReadings: state.nodeReadings.map(nr =>
                nr.nodeId === nodeId
                  ? { ...nr, anomalyCount, anomalySeverity: severity !== 'Info' ? severity : nr.anomalySeverity }
                  : nr
              ),
            };
          });
        });
        set({ _anomalyUnsub: unsubAnomaly });
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
        const raw = await aiApi.assess(zone.id);
        const scores = (raw.scores as Record<string, number>) || {};
        const overall = scores.overall ?? 85;
        assessments[zone.id] = {
          score: overall,
          status: overall >= 80 ? 'normal' : overall >= 60 ? 'warning' : 'danger',
          summary: raw.deviations ? `存在 ${(raw.deviations as unknown[]).length} 项偏离` : '各项指标正常',
          details: ((raw.deviations as Array<{ param: string; current: number; optimal: number }>) || [])
            .map(d => `${d.param}: 当前 ${d.current}, 最优 ${d.optimal}`),
        };
      } catch {
        // skip failed assessments
      }
    }
    const healthScore = calcHealthScore(assessments);
    const state = get();
    const prevScore = state.healthScore;
    const prevAssessments = state.assessments;
    const isFirstReal = Object.keys(prevAssessments).length === 0 || prevScore === 85;
    const healthTrend = isFirstReal ? 0 : healthScore - prevScore;
    const todoItems = buildTodoItems(zones, assessments);
    set({ assessments, healthScore, healthTrend, todoItems });
  },

  fetchEmergencies: async () => {
    try {
      const data = await aiApi.emergencyStatus();
      set({ emergencies: (data.active_emergencies || []).map(e => ({
        id: `${e.type}-${e.triggered_at}`,
        type: e.type,
        message: e.message,
        severity: e.confidence > 0.8 ? 'critical' as const : 'high' as const,
        timestamp: new Date(e.triggered_at * 1000).toISOString(),
      })) });
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
    get()._realtimeUnsub?.();
    get()._statusUnsub?.();
    get()._anomalyUnsub?.();
    clearTimeout(get()._assessTimer);
    set({ _realtimeUnsub: null, _statusUnsub: null, _anomalyUnsub: null, _assessTimer: undefined });
  },
}));
