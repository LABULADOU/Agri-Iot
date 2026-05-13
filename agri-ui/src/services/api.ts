import axios from 'axios';
import type {
  Zone, SensorNode, SensorReading, AggregatedReading,
  AccumulatedTemp, WeatherData, ControlCommand, Device, Rule, QueryParams
} from '../types';

const api = axios.create({
  baseURL: '/api/v1',
  timeout: 10000,
});

api.interceptors.response.use(
  res => res.data,
  err => {
    console.error('API Error:', err);
    return Promise.reject(err);
  }
);

// Zone APIs
export const zoneApi = {
  list: () => api.get<Zone[]>('/zones').then(res => res.data as Zone[]),
  get: (id: string) => api.get<Zone>(`/zones/${id}`).then(res => res.data as Zone),
  create: (data: Partial<Zone>) => api.post<Zone>('/zones', data).then(res => res.data as Zone),
  update: (id: string, data: Partial<Zone>) => api.put<Zone>(`/zones/${id}`, data).then(res => res.data as Zone),
  delete: (id: string) => api.delete(`/zones/${id}`),
};

// Sensor Node APIs
export const nodeApi = {
  list: (zoneId?: string) => api.get<SensorNode[]>('/nodes', { params: zoneId ? { zone_id: zoneId } : undefined }).then(res => res.data as SensorNode[]),
  get: (id: string) => api.get<SensorNode>(`/nodes/${id}`).then(res => res.data as SensorNode),
  create: (data: Partial<SensorNode>) => api.post<SensorNode>('/nodes', data).then(res => res.data as SensorNode),
  update: (id: string, data: Partial<SensorNode>) => api.put<SensorNode>(`/nodes/${id}`, data).then(res => res.data as SensorNode),
  delete: (id: string) => api.delete(`/nodes/${id}`),
  getReadings: (id: string, params: { metric?: string; start?: string; end?: string; limit?: number }) =>
    api.get<SensorReading[]>(`/nodes/${id}/readings`, { params }).then(res => res.data as SensorReading[]),
};

// Aggregated Data APIs
export const dataApi = {
  query: (params: QueryParams) => api.get<AggregatedReading[]>('/readings/aggregated', { params }).then(res => res.data as AggregatedReading[]),
};

// Accumulated Temperature APIs
export const accTempApi = {
  list: (zoneId: string, params?: { start?: string; end?: string }) =>
    api.get<AccumulatedTemp[]>(`/zones/${zoneId}/accumulated-temp`, { params }).then(res => res.data as AccumulatedTemp[]),
};

// Weather APIs
export const weatherApi = {
  getNow: (location?: string) => api.get<WeatherData>('/weather/now', { params: { location } }).then(res => res.data as WeatherData),
  getForecast: (days: number = 3) => api.get<WeatherData>('/weather/forecast', { params: { days } }).then(res => res.data as WeatherData),
};

// Control APIs
export const controlApi = {
  sendCommand: (data: ControlCommand) => api.post('/control/command', data),
  getStatus: (nodeId: string) => api.get(`/nodes/${nodeId}/controls`),
};

// Device APIs
export const deviceApi = {
  list: () => api.get<Device[]>('/devices').then(res => res.data as Device[]),
  get: (id: string) => api.get<Device>(`/devices/${id}`).then(res => res.data as Device),
  sendCommand: (id: string, command: string, payload?: Record<string, unknown>) =>
    api.post(`/devices/${id}/command`, { command, payload }),
};

// Rule APIs
export const ruleApi = {
  list: () => api.get<Rule[]>('/rules').then(res => res.data as Rule[]),
  get: (id: string) => api.get<Rule>(`/rules/${id}`).then(res => res.data as Rule),
  create: (data: Partial<Rule>) => api.post<Rule>('/rules', data).then(res => res.data as Rule),
  update: (id: string, data: Partial<Rule>) => api.put<Rule>(`/rules/${id}`, data).then(res => res.data as Rule),
  delete: (id: string) => api.delete(`/rules/${id}`),
};

export default api;