import axios from 'axios';
import type {
  Zone, SensorNode, SensorReading, AggregatedReading,
  HourlyPrecip, GeoCity,
  Device, Rule, QueryParams,
  EmergencyStatusResponse, KnowledgeSearchResult, ControlCaseRecord, AgentResponse, KnowledgeNoteMeta,
  VarietyResponse,
} from '../types';

const api = axios.create({
  baseURL: '/api/v1',
  timeout: 10000,
});

export const apiLong = axios.create({
  baseURL: '/api/v1',
  timeout: 120000,
});

// Zone APIs → /areas
export const zoneApi = {
  list: () => api.get<Zone[]>('/areas').then(res => res.data),
  get: (id: string) => api.get<Zone>(`/areas/${id}`).then(res => res.data),
  create: (data: Partial<Zone>) => api.post<Zone>('/areas', data).then(res => res.data),
  update: (id: string, data: Partial<Zone>) => api.put<Zone>(`/areas/${id}`, data).then(res => res.data),
  delete: (id: string) => api.delete(`/areas/${id}`),
};

// Sensor Node APIs → /devices
export const nodeApi = {
  list: (zoneId?: string) => api.get<SensorNode[]>('/devices', { params: zoneId ? { area_id: zoneId } : undefined }).then(res => res.data),
  get: (id: string) => api.get<SensorNode>(`/devices/${id}`).then(res => res.data),
  create: (data: Partial<SensorNode>) => api.post<SensorNode>('/devices', data).then(res => res.data),
  update: (id: string, data: Partial<SensorNode>) => api.put<SensorNode>(`/devices/${id}`, data).then(res => res.data),
  delete: (id: string) => api.delete(`/devices/${id}`),
  getReadings: (id: string, params: { metric?: string; start?: string; end?: string; limit?: number }) =>
    apiLong.get<SensorReading[]>(`/devices/${id}/readings`, { params }).then(res => res.data),
};

// Aggregated Data APIs
export const dataApi = {
  query: async (params: QueryParams) => {
    if (!params.node_id) return [];
    const metrics = params.metric ? params.metric.split(',') : [];
    const allResults: AggregatedReading[] = [];
    for (const metric of metrics) {
      const queryParams: Record<string, string> = {
        device_id: params.node_id,
        metric,
        period: params.period || 'hour',
      };
      if (params.start) queryParams.start = String(Math.floor(new Date(params.start).getTime() / 1000));
      if (params.end) queryParams.end = String(Math.floor(new Date(params.end).getTime() / 1000));
      const raw = await api.get<AggregatedReading[]>('/readings/aggregate', { params: queryParams });
      allResults.push(...raw.data.map(r => ({
        ...r,
        timestamp: typeof r.timestamp === 'number'
          ? new Date((r.timestamp as number) * 1000).toISOString()
          : r.timestamp,
      })));
    }
    return allResults;
  },
};

// Weather APIs — returns raw QWeather JSON from backend proxy
export const weatherApi = {
  getNow: (location: string = '39.92,116.41') =>
    api.get<{ code: string; now: Record<string, string> }>('/weather/now', { params: { location } }).then(res => res.data),
  getForecast3d: (location: string = '39.92,116.41') =>
    api.get<{ code: string; daily: Record<string, string>[] }>('/weather/3d', { params: { location } }).then(res => res.data),
  getMinutely: (location: string = '101010100') =>
    api.get<{ code: string; summary: string; hourly: HourlyPrecip[] }>('/weather/minutely', { params: { location } }).then(res => res.data),
  getWarning: (location: string = '101010100') =>
    api.get<{ code: string; warning?: Array<Record<string, string>> }>('/weather/warning', { params: { location } }).then(res => res.data),
  geoLookup: (query: string, number: number = 10) =>
    api.get<{ location: GeoCity[] }>('/weather/geo', { params: { location: query, number } }).then(res => res.data),
};

// Device APIs
export const deviceApi = {
  list: () => api.get<Device[]>('/devices').then(res => res.data),
  get: (id: string) => api.get<Device>(`/devices/${id}`).then(res => res.data),
  sendCommand: (id: string, command: string, params?: Record<string, unknown>) =>
    api.post(`/devices/${id}/command`, { command, params }),
};

// Rule APIs
export const ruleApi = {
  list: () => api.get<Rule[]>('/rules').then(res => res.data),
  get: (id: string) => api.get<Rule>(`/rules/${id}`).then(res => res.data),
  create: (data: Partial<Rule>) => api.post<Rule>('/rules', data).then(res => res.data),
  update: (id: string, data: Partial<Rule>) => api.put<Rule>(`/rules/${id}`, data).then(res => res.data),
  delete: (id: string) => api.delete(`/rules/${id}`),
};

// AI Decision APIs
export const aiApi = {
  assess: (areaId: string) =>
    api.post<Record<string, unknown>>('/ai/assess', { area_id: areaId }).then(res => res.data),
  emergencyStatus: () =>
    api.get<EmergencyStatusResponse>('/ai/emergency/status').then(res => res.data),
  knowledgeSearch: (query: string) =>
    api.get<KnowledgeSearchResult[]>('/ai/knowledge/search', { params: { query } }).then(res => res.data),
  knowledgeCases: (limit?: number) =>
    api.get<ControlCaseRecord[]>('/ai/knowledge/cases', { params: { limit } }).then(res => res.data),
  listKnowledgeBase: () =>
    api.get<{ notes: KnowledgeNoteMeta[] }>('/ai/knowledge/obsidian/list').then(res => res.data),
  readNote: (path: string) =>
    api.get<{ path: string; content: string }>('/ai/knowledge/obsidian/note', { params: { path } }).then(res => res.data),
  chrysanthemumVarieties: () =>
    api.get<VarietyResponse>('/ai/knowledge/chrysanthemum').then(res => res.data),
  agentQuery: (query: string, history?: { role: string; content: string }[]) =>
    apiLong.post<AgentResponse>('/ai/agent/query', { query, history }).then(res => res.data),
  agentQueryStream: async (query: string, onChunk: (text: string) => void, onDone: (resp: AgentResponse) => void, signal?: AbortSignal, history?: { role: string; content: string }[]) => {
    const res = await fetch('/api/v1/ai/agent/query', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ query, history }),
      signal,
    });
    const data: AgentResponse = await res.json();
    onChunk(data.answer);
    onDone(data);
  },
};

// Farm Log APIs
export const farmApi = {
  listOps: (params?: { area_id?: string; date_from?: string; date_to?: string; category?: string; page?: number; limit?: number }) =>
    api.get<{ operations: import('../types').FarmOperation[]; page: number; limit: number }>('/farm/operations', { params }).then(res => res.data),
  getOp: (id: string) =>
    api.get<import('../types').FarmOperation>(`/farm/operations/${id}`).then(res => res.data),
  createOp: (data: {
    area_id: string; log_date: string; category: string; content: string;
    log_time?: string; operator?: string; weather?: string; crop_status?: string;
    notes?: string; details?: Record<string, unknown>;
  }) => api.post('/farm/operations', data).then(res => res.data),
  updateOp: (id: string, data: Record<string, unknown>) =>
    api.put(`/farm/operations/${id}`, data).then(res => res.data),
  deleteOp: (id: string) => api.delete(`/farm/operations/${id}`),
  listTemplates: (category?: string) =>
    api.get<import('../types').FarmOpTemplate[]>('/farm/templates', { params: { category } }).then(res => res.data),
  createTemplate: (data: { name: string; category: string; details?: Record<string, unknown>; sort_order?: number }) =>
    api.post('/farm/templates', data).then(res => res.data),
  updateTemplate: (id: string, data: Record<string, unknown>) =>
    api.put(`/farm/templates/${id}`, data).then(res => res.data),
  deleteTemplate: (id: string) => api.delete(`/farm/templates/${id}`),
};

// Inventory APIs
export const inventoryApi = {
  listItems: (params?: { category?: string; low_stock?: boolean; search?: string }) =>
    api.get<{ items: import('../types').InventoryItem[] }>('/inventory/items', { params }).then(res => res.data),
  getItem: (id: string) =>
    api.get<import('../types').InventoryItem>(`/inventory/items/${id}`).then(res => res.data),
  createItem: (data: Record<string, unknown>) => api.post('/inventory/items', data).then(res => res.data),
  updateItem: (id: string, data: Record<string, unknown>) =>
    api.put(`/inventory/items/${id}`, data).then(res => res.data),
  deleteItem: (id: string) => api.delete(`/inventory/items/${id}`),
  listTransactions: (itemId: string, params?: { limit?: number; offset?: number }) =>
    api.get<{ transactions: import('../types').InventoryTransaction[] }>(`/inventory/items/${itemId}/transactions`, { params }).then(res => res.data),
  createTransaction: (data: { item_id: string; txn_type: 'in' | 'out' | 'adjust'; quantity: number; operator?: string; related_type?: string; related_id?: string; note?: string }) =>
    api.post<{ id: string; new_stock: number }>('/inventory/transactions', data).then(res => res.data),
  getSummary: () =>
    api.get<{ summary: { total_items: number; total_value: number; low_stock_count: number }; categories: Array<{ category: string; count: number; total_value: number; low_count: number }> }>('/inventory/summary').then(res => res.data),
};

// Yield APIs
export const yieldApi = {
  listHarvests: (params?: { area_id?: string; crop_batch_id?: string; date_from?: string; date_to?: string; page?: number; limit?: number }) =>
    api.get<{ harvests: import('../types').Harvest[]; page: number; limit: number }>('/yield/harvests', { params }).then(res => res.data),
  createHarvest: (data: Record<string, unknown>) => api.post('/yield/harvests', data).then(res => res.data),
  updateHarvest: (id: string, data: Record<string, unknown>) => api.put(`/yield/harvests/${id}`, data).then(res => res.data),
  deleteHarvest: (id: string) => api.delete(`/yield/harvests/${id}`),
  getAnalysis: (params?: { area_id?: string; crop_id?: string; date_from?: string; date_to?: string }) =>
    api.get<import('../types').YieldAnalysis>('/yield/analysis', { params }).then(res => res.data),
};

// Mixing (配方) APIs
export const mixingApi = {
  recommendFertilizer: (data: { area_id: string; crop_batch_id?: string; growth_days?: number }) =>
    api.post<import('../types').MixingPlan>('/mixing/fertilizer/recommend', data).then(res => res.data),
  recommendPesticide: (data: { area_id: string; crop_batch_id?: string; target_pest: string }) =>
    api.post<import('../types').MixingPlan>('/mixing/pesticide/recommend', data).then(res => res.data),
  listRecipes: (params?: { area_id?: string; mix_type?: string; page?: number; limit?: number }) =>
    api.get<{ recipes: import('../types').MixingRecipe[]; page: number; limit: number }>('/mixing/recipes', { params }).then(res => res.data),
  applyRecipe: (id: string) => api.post<{ farm_op_id: string; warnings: string[] }>(`/mixing/recipes/${id}/apply`).then(res => res.data),
  listPresets: (mixType?: string) =>
    api.get<{ presets: import('../types').MixingPreset[] }>('/mixing/presets', { params: { mix_type: mixType } }).then(res => res.data),
  createPreset: (data: Record<string, unknown>) => api.post('/mixing/presets', data).then(res => res.data),
  updatePreset: (id: string, data: Record<string, unknown>) => api.put(`/mixing/presets/${id}`, data).then(res => res.data),
  deletePreset: (id: string) => api.delete(`/mixing/presets/${id}`),
};

// Labor (用工) APIs
export const laborApi = {
  listRecords: (params?: { date_from?: string; date_to?: string; area_id?: string; category?: string; page?: number; limit?: number }) =>
    api.get<{ records: import('../types').LaborRecord[]; page: number; limit: number; total: number }>('/labor/records', { params }).then(res => res.data),
  createRecord: (data: Record<string, unknown>) => api.post('/labor/records', data).then(res => res.data),
  updateRecord: (id: string, data: Record<string, unknown>) => api.put(`/labor/records/${id}`, data).then(res => res.data),
  deleteRecord: (id: string) => api.delete(`/labor/records/${id}`),
  getSummary: (params?: { date_from?: string; date_to?: string; area_id?: string }) =>
    api.get<{ summary: import('../types').LaborSummary }>('/labor/summary', { params }).then(res => res.data),
};

export default api;