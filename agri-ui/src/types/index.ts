export type ComfortLevel = 'optimal' | 'warning' | 'danger';

export interface ComfortConfig {
  airTemp: { min: number; max: number };
  airHumidity: { min: number; max: number };
  soilTemp: { min: number; max: number };
  soilMoisture: { min: number; max: number };
  ecValue: { min: number; max: number };
}

// Backend Area model: id, name, description?, created_at
export interface Zone {
  id: string;
  name: string;
  description?: string;
  cropType?: string;
  comfortConfig?: ComfortConfig;
  nodeIds?: string[];
  created_at?: string;
}

// Backend Device model (snake_case from API)
export interface SensorNode {
  id: string;
  name: string;
  node_id: string;
  device_type: 'sensor' | 'actuator';
  status: 'online' | 'offline' | 'error';
  area_id?: string;
  capabilities?: string[];
  config?: Record<string, unknown>;
  comfort_config?: Record<string, unknown>;
  created_at?: string;
  updated_at?: string;
  // Frontend-only extensions (optional defaults)
  hasIrrigation?: boolean;
  hasSideVent?: boolean;
  hasRoofVent?: boolean;
  ventRange?: { min: number; max: number };
  lastSeen?: string;
}

export interface Sensor {
  id: string;
  metric: string;
  name: string;
  unit: string;
  value: number | null;
  status: 'ok' | 'error' | 'offline';
}

export interface SensorReading {
  id: number;
  device_id: string;
  metric: string;
  value: number;
  unit: string;
  timestamp: string;
}

export interface AggregatedReading {
  timestamp: string;
  metric: string;
  node_id?: string;
  max: number;
  min: number;
  avg: number;
  count: number;
}

export interface AccumulatedTemp {
  id: string;
  zoneId: string;
  date: string;
  accumulated: number;
  threshold: number;
}

export interface Assessment {
  score: number;
  status: 'normal' | 'warning' | 'danger';
  summary: string;
  details?: string[];
}

export interface ControlCase {
  id: string;
  title: string;
  summary: string;
  date: string;
}

export interface Emergency {
  id: string;
  type: string;
  message: string;
  severity: 'high' | 'critical';
  timestamp: string;
}

export interface TodoItem {
  id: string;
  zoneId?: string;
  zoneName: string;
  type: 'warning' | 'attention' | 'offline';
  message: string;
  aiRecommendation?: string;
  timestamp: string;
  actionable: boolean;
}

export interface AIRecommendation {
  id: string;
  content: string;
  targetArea: string;
  caseLink?: string;
}

// Raw QWeather API response shapes (returned by backend proxy)
export interface QWeatherNow {
  temp: string;
  feelsLike: string;
  icon: string;
  text: string;
  wind360: string;
  windDir: string;
  windScale: string;
  windSpeed: string;
  humidity: string;
  precip: string;
  pressure: string;
  vis: string;
  cloud: string;
  dew: string;
}

export interface QWeatherDaily {
  fxDate: string;
  tempMax: string;
  tempMin: string;
  iconDay: string;
  textDay: string;
  iconNight: string;
  textNight: string;
  windDirDay: string;
  windScaleDay: string;
  humidity: string;
  precip: string;
}

export interface QWeatherWarning {
  id: string;
  pubTime: string;
  title: string;
  level: string;
  type: string;
  text: string;
}

// Normalized frontend weather state
export interface WeatherData {
  temp: number;
  feelsLike: number;
  text: string;
  icon: string;
  humidity: number;
  windDir: string;
  windScale: string;
  windSpeed: number;
  precip: number;
  updateTime: string;
}

export interface WeatherForecastDay {
  date: string;
  tempMax: number;
  tempMin: number;
  textDay: string;
  iconDay: string;
  windDirDay: string;
  windScaleDay: string;
}

export interface WeatherWarning {
  title: string;
  level: string;
  type: string;
  pubTime: string;
}

export interface GeoCity {
  name: string;
  id: string;
  adm1: string;
  adm2: string;
}

export interface HourlyPrecip {
  time: string;
  text: string;
  temp: string;
  precip: string;
  pop: string;
}

export interface MinutelyForecast {
  summary: string;
  hourly: HourlyPrecip[];
}

export interface CityLocation {
  name: string;
  id: string;
  adm1: string;
  country: string;
}

export interface ControlCommand {
  deviceId: string;
  command: 'irrigation' | 'side_vent' | 'roof_vent';
  action: 'on' | 'off' | number;
}

export interface Device {
  id: string;
  name: string;
  node_id: string;
  device_type: 'sensor' | 'actuator';
  status: 'online' | 'offline' | 'error';
  area_id?: string;
  capabilities?: string[];
  config?: Record<string, unknown>;
  comfort_config?: Record<string, unknown>;
  created_at?: string;
  updated_at?: string;
}

export interface Rule {
  id: string;
  name: string;
  enabled: boolean;
  trigger_type?: string;
  triggerType?: string;
  conditions?: Condition[];
  actions?: Action[];
  schedule?: string;
  priority?: number;
  auto_execute?: boolean;
  created_at?: string;
  createdAt?: string;
}

export interface Condition {
  metric: string;
  operator: '>' | '<' | '>=' | '<=' | '==';
  value: number;
  nodeId?: string;
}

export interface Action {
  deviceId: string;
  command: string;
  payload?: Record<string, unknown>;
}

export type TimePeriod = 'hour' | 'day' | 'week' | 'month' | 'custom';

export interface QueryParams {
  node_id?: string;
  metric?: string;
  period: TimePeriod;
  start?: string;
  end?: string;
}