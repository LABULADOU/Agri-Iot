export type ComfortLevel = 'optimal' | 'warning' | 'danger';

export interface ComfortConfig {
  airTemp: { min: number; max: number };
  airHumidity: { min: number; max: number };
  soilTemp: { min: number; max: number };
  soilMoisture: { min: number; max: number };
  ecValue: { min: number; max: number };
}

export interface Zone {
  id: string;
  name: string;
  description: string;
  location: string;
  cropType: string;
  comfortConfig: ComfortConfig;
  nodeIds: string[];
  createdAt: string;
  updatedAt: string;
}

export interface SensorNode {
  id: string;
  name: string;
  zoneId: string;
  hasIrrigation: boolean;
  hasSideVent: boolean;
  hasRoofVent: boolean;
  ventRange: { min: number; max: number };
  sensors: Sensor[];
  status: 'online' | 'offline' | 'error';
  lastSeen: string;
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
  deviceId: string;
  nodeId: string;
  metric: string;
  value: number;
  unit: string;
  timestamp: string;
}

export interface AggregatedReading {
  timestamp: string;
  metric: string;
  nodeId: string;
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

export interface WeatherData {
  location: string;
  temp: number;
  humidity: number;
  text: string;
  windSpeed: number;
  windDir: string;
  updateTime: string;
  forecast: WeatherForecast[];
}

export interface WeatherForecast {
  date: string;
  tempMax: number;
  tempMin: number;
  textDay: string;
  textNight: string;
  humidity: number;
}

export interface ControlCommand {
  deviceId: string;
  command: 'irrigation' | 'side_vent' | 'roof_vent';
  action: 'on' | 'off' | number;
}

export interface Device {
  id: string;
  name: string;
  nodeId: string;
  type: 'sensor' | 'actuator';
  status: 'online' | 'offline' | 'error';
  config?: Record<string, unknown>;
  createdAt: string;
  updatedAt: string;
}

export interface Rule {
  id: string;
  name: string;
  enabled: boolean;
  triggerType: 'schedule' | 'condition';
  conditions: Condition[];
  actions: Action[];
  schedule?: string;
  createdAt: string;
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
  nodeId?: string;
  metric?: string;
  period: TimePeriod;
  start?: string;
  end?: string;
}