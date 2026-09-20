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
  timestamp: number | string;
}

export interface AggregatedReading {
  timestamp: number | string;
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
  pressure: number;
  vis: number;
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
  country?: string;
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

export interface EmergencyStatusResponse {
  active_emergencies: EmergencyRuleResponse[];
  night_mode_active: boolean;
  pauses_auto_mode: boolean;
}

export interface EmergencyRuleResponse {
  type: string;
  confidence: number;
  message: string;
  triggered_at: number;
  pauses_auto_mode: boolean;
  night_additional_contact: boolean;
}

export interface KnowledgeSearchResult {
  type: 'crop_profile' | 'pest_knowledge' | 'weather_knowledge';
  id: string;
  name: string;
  condition_type?: string;
  data: Record<string, unknown>;
}

export interface ControlCaseRecord {
  id: string;
  area_id?: string;
  crop_profile_id?: string;
  situation?: string;
  weather_forecast?: string;
  action_taken?: string;
  manual_override?: number;
  outcome?: string;
  effect_rating?: number;
  health_improvement?: number;
  action_duration_minutes?: number;
  recovery_time_minutes?: number;
  notes?: string;
  timestamp: number;
  embedding_id?: string;
}

export interface AgentResponse {
  answer: string;
  data_sources: string[];
  follow_up_questions: string[];
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'agent';
  content: string;
  timestamp: number;
  data_sources?: string[];
  follow_up_questions?: string[];
}

export interface KnowledgeNoteMeta {
  path: string;
  title: string;
  knowledge_type?: string;
  适用作物?: string;
  知识领域?: string;
  置信度?: string;
}

export interface KnowledgeNote extends KnowledgeNoteMeta {
  content: string;
}

export interface ChrysanthemumVariety {
  name: string;
  growth: string;
  weeks: string;
  color: string;
  flower_type: string;
  cold_tolerant: string;
  heat_tolerant: string;
  disease_resistance: string;
}

export interface VarietyResponse {
  varieties: ChrysanthemumVariety[];
}

export type TimePeriod = 'hour' | 'day' | 'week' | 'month' | '10min' | 'custom';

export type ViewMode = 'ten_min' | 'daily' | 'realtime';

export interface QueryParams {
  node_id?: string;
  metric?: string;
  period: TimePeriod;
  start?: string;
  end?: string;
}

// ====== 农事操作日志 ======

export type FarmOpCategory = '打药' | '施肥' | '灌溉' | '修剪' | '采收' | '设备维护' | '定植' | '育苗' | '巡棚' | '其他';

export type FarmOpStatus = 'planned' | 'in_progress' | 'completed' | 'cancelled';

export interface PesticideItem {
  formulation: string;
  ingredient: string;
  brand: string;
  reg_no: string;
  dosage: string;
  dosage_per_unit: string;
  /** 本次总用量（数字），勾选库存联动时用于自动出库 */
  usage?: number;
  /** 本次总用量单位，如 kg / g / L / ml */
  usage_unit?: string;
}

export interface PesticideDetails {
  items: PesticideItem[];
  water_volume: string;
  target_pest: string;
  application_method: string;
}

export interface FertilizerItem {
  name: string;
  amount: string;
  n: string;
  p: string;
  k: string;
}

export interface FertilizerDetails {
  items: FertilizerItem[];
  method: string;
  total_volume: string;
  ec: string;
  ph: string;
}

export interface FarmOperation {
  id: string;
  area_id: string;
  log_date: string;
  log_time: string;
  category: FarmOpCategory;
  content: string;
  operator: string;
  status: FarmOpStatus;
  weather: string;
  crop_status: string;
  notes: string;
  details: PesticideDetails | FertilizerDetails | Record<string, unknown>;
  created_at: number;
  updated_at: number;
}

export interface FarmOpTemplate {
  id: string;
  name: string;
  category: FarmOpCategory;
  details: PesticideDetails | FertilizerDetails | Record<string, unknown>;
  sort_order: number;
  created_at: number;
}

export interface FarmOperationsListResponse {
  operations: FarmOperation[];
  page: number;
  limit: number;
}

// ====== 每日用工成本 ======

export interface LaborRecord {
  id: string;
  log_date: string;
  area_id: string | null;
  category: string;
  worker: string;
  worker_count: number;
  work_hours: number;
  rate: number;
  amount: number;
  paid_status: string;
  operator: string;
  notes: string;
  created_at: number;
  updated_at: number;
}

export interface LaborSummary {
  total_amount: number;
  total_worker_days: number;
  total_workdays: number;
  record_count: number;
  by_date: Array<{ log_date: string; amount: number; worker_count: number }>;
  by_category: Array<{ category: string; amount: number; count: number }>;
}

export interface AnomalyEvent {
  node_id: string;
  metric: string;
  anomaly_type: 'Dht22Fault' | 'MetricSilent' | 'RateAnomaly' | 'SpatialAnomaly';
  severity: 'Info' | 'Warning' | 'Critical';
  value_original?: number;
  message: string;
  timestamp: number;
}
// ==================== Inventory ====================
export type InventoryCategory = 'seed' | 'fertilizer' | 'pesticide' | 'materiel' | 'other';

export interface InventoryItem {
  id: string;
  name: string;
  category: InventoryCategory;
  unit: string;
  price: number;
  stock: number;
  warning_threshold: number;
  low_stock: boolean;
  batch_no: string;
  expiry_date: string;
  manufacturer: string;
  specs: string;
  notes: string;
  created_at: number;
  updated_at: number;
}

export interface InventoryTransaction {
  id: string;
  item_id: string;
  item_name: string;
  txn_type: 'in' | 'out' | 'adjust';
  quantity: number;
  operator: string;
  related_type: string;
  related_id: string;
  note: string;
  created_at: number;
}

// ==================== Yield ====================
export interface Harvest {
  id: string;
  area_id: string;
  area_name: string;
  crop_batch_id?: string;
  crop_name?: string;
  harvest_date: string;
  quantity: number;
  unit: string;
  grade: string;
  price: number;
  amount: number;
  operator: string;
  notes: string;
  created_at: number;
}

export interface YieldAnalysis {
  yield: {
    total_quantity: number;
    total_revenue: number;
    total_harvests: number;
    input_cost_estimate: number;
    net_profit_estimate: number;
  };
  areas: Array<{ area_id: string; area_name: string; total_quantity: number; total_amount: number; harvest_count: number }>;
  trend: Array<{ date: string; quantity: number; amount: number }>;
  grades: Array<{ grade: string; quantity: number; amount: number }>;
  operations: Record<string, number>;
}

// ==================== Mixing ====================
export interface MixingPlan {
  type: 'fertilizer' | 'pesticide';
  stage?: string;
  stage_key?: string;
  growth_days?: number;
  target_pest?: string;
  severity?: string;
  treatment?: string;
  source: 'preset' | 'knowledge' | 'fallback' | 'base';
  plan: {
    items: Array<{ name: string; n?: number; p?: number; k?: number; amount?: number; unit?: string; dilution?: string }>;
    dilution?: string;
    water_volume?: number | string;
    ec_target?: number;
    safety_interval_days?: number;
  };
  adjustments: string[];
  reasoning: string;
}

export interface MixingRecipe {
  id: string;
  area_id: string;
  area_name: string;
  crop_batch_id?: string;
  mix_type: 'fertilizer' | 'pesticide';
  stage_key: string;
  growth_days: number;
  result: MixingPlan;
  status: 'generated' | 'applied';
  farm_op_id: string;
  created_at: number;
}

export interface MixingPreset {
  id: string;
  crop_id?: string;
  mix_type: 'fertilizer' | 'pesticide';
  stage_key: string;
  name: string;
  items: Array<{ name: string; dilution?: string; amount?: number; unit?: string }>;
  dilution: string;
  dosage_per_unit: string;
  water_volume: string;
  ec_target?: number;
  safety_interval_days: number;
  note: string;
  created_at: number;
}
