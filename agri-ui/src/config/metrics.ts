export const METRICS = [
  'temperature',
  'humidity',
  'soil_temperature',
  'soil_moisture',
  'ec',
  'light',
  'rssi',
  'relay_state',
] as const;
export type MetricKey = typeof METRICS[number];

export interface MetricConfig {
  label: string;
  unit: string;
  color: string;
  min?: number;
  max?: number;
  maxScale?: number;
}

export const METRIC_CONFIG: Record<string, MetricConfig> = {
  temperature:      { label: '空气温度', unit: '℃',     color: '#22C55E', min: 18, max: 28, maxScale: 50 },
  humidity:         { label: '空气湿度', unit: '%',     color: '#0EA5E9', min: 60, max: 80, maxScale: 100 },
  soil_temperature: { label: '土壤温度', unit: '℃',     color: '#F59E0B', min: 15, max: 25, maxScale: 50 },
  soil_moisture:    { label: '土壤湿度', unit: '%',     color: '#06B6D4', min: 40, max: 70, maxScale: 100 },
  ec:               { label: 'EC值',    unit: 'mS/cm', color: '#8B5CF6', min: 1.5, max: 3.5, maxScale: 5 },
  light:            { label: '光照',    unit: 'lux',   color: '#F59E0B', min: 0,  max: 200000, maxScale: 200000 },
  rssi:             { label: 'WiFi信号', unit: 'dBm',  color: '#EF4444' },
  relay_state:      { label: '继电器',  unit: '',       color: '#F97316' },
};

export function getMetricLabel(key: string): string {
  return METRIC_CONFIG[key]?.label ?? key;
}

export function getMetricColor(key: string): string {
  return METRIC_CONFIG[key]?.color ?? '#999';
}

export const metricSelectOptions = METRICS
  .filter(m => m !== 'relay_state' && m !== 'light' && m !== 'rssi')
  .map(m => ({ value: m, label: METRIC_CONFIG[m].label }));
