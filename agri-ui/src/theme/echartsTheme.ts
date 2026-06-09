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

export const metricColors: Record<string, string> = {
  temperature: '#22C55E',
  humidity: '#0EA5E9',
  soil_temperature: '#F59E0B',
  soil_moisture: '#06B6D4',
  ec: '#8B5CF6',
  rssi: '#EF4444',
  relay_state: '#F97316',
};

export const metricLabels: Record<string, string> = {
  temperature: '空气温度',
  humidity: '空气湿度',
  soil_temperature: '土壤温度',
  soil_moisture: '土壤湿度',
  ec: 'EC值',
  rssi: 'WiFi信号',
  relay_state: '继电器状态',
};

export const chartGrid = {
  left: '3%',
  right: '4%',
  bottom: '15%',
  top: '3%',
  containLabel: true,
};
