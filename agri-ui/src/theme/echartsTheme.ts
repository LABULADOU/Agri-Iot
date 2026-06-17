import { METRIC_CONFIG } from '../config/metrics';

export { METRIC_CONFIG as metricConfig };

export const CHART_COLORS = {
  primary: '#22C55E',
  info: '#0EA5E9',
  warning: '#F59E0B',
  danger: '#EF4444',
  purple: '#8B5CF6',
  gray100: '#F3F4F6',
  gray200: '#E5E7EB',
  gray400: '#9CA3AF',
  gray500: '#6B7280',
};

export const metricColors: Record<string, string> = Object.fromEntries(
  Object.entries(METRIC_CONFIG).map(([k, v]) => [k, v.color])
);

export const metricLabels: Record<string, string> = Object.fromEntries(
  Object.entries(METRIC_CONFIG).map(([k, v]) => [k, v.label])
);

export const chartGrid = {
  left: '3%',
  right: '4%',
  bottom: '15%',
  top: '3%',
  containLabel: true,
};
