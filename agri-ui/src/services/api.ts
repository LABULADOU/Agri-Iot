import axios from 'axios';
import type {
  Zone, SensorNode, SensorReading, AggregatedReading,
  WeatherData, WeatherForecastDay, WeatherWarning, MinutelyForecast, HourlyPrecip,
  Device, Rule, QueryParams
} from '../types';

const api = axios.create({
  baseURL: '/api/v1',
  timeout: 10000,
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
    api.get<SensorReading[]>(`/devices/${id}/readings`, { params }).then(res => res.data),
};

// Aggregated Data APIs
export const dataApi = {
  query: async (params: QueryParams) => {
    if (!params.node_id) return [];
    const queryParams: Record<string, string> = {};
    if (params.start) queryParams.start = String(Math.floor(new Date(params.start).getTime() / 1000));
    if (params.end) queryParams.end = String(Math.floor(new Date(params.end).getTime() / 1000));
    queryParams.limit = '5000';
    const raw = await api.get<SensorReading[]>(`/devices/${params.node_id}/readings`, { params: queryParams });
    const readings = raw.data;
    const result: AggregatedReading[] = readings
      .filter(r => r.metric && r.value !== null)
      .map(r => ({
        timestamp: typeof r.timestamp === 'number'
          ? new Date((r.timestamp as number) * 1000).toISOString()
          : r.timestamp,
        metric: r.metric,
        max: r.value,
        min: r.value,
        avg: r.value,
        count: 1,
      }));
    return result;
  },
};

// Weather APIs — returns raw QWeather JSON from backend proxy
export const weatherApi = {
  getNow: (location: string = '101010100') =>
    api.get<{ code: string; now: Record<string, string> }>('/weather/now', { params: { location } }).then(res => res.data),
  getForecast3d: (location: string = '101010100') =>
    api.get<{ code: string; daily: Record<string, string>[] }>('/weather/3d', { params: { location } }).then(res => res.data),
  getMinutely: (location: string = '101010100') =>
    api.get<{ code: string; summary: string; hourly: HourlyPrecip[] }>('/weather/minutely', { params: { location } }).then(res => res.data),
  getWarning: (location: string = '101010100') =>
    api.get<{ code: string; warning?: Array<Record<string, string>> }>('/weather/warning', { params: { location } }).then(res => res.data),
};

// Control APIs → use device command endpoint
export const controlApi = {
  sendCommand: (deviceId: string, command: string, params?: Record<string, unknown>) =>
    api.post(`/devices/${deviceId}/command`, { command, params }),
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

export default api;