const METRIC_LABEL = {
  temperature: { icon: '\uD83C\uDF21\uFE0F', label: '\u2103', fixed: 1 },
  humidity: { icon: '\uD83D\uDCA7', label: '%', fixed: 0 },
  soil_temperature: { icon: '\uD83C\uDF31', label: '\u2103', fixed: 1 },
  soil_moisture: { icon: '\uD83D\uDCA7', label: '%', fixed: 0 },
};

function formatMetric(metric, value) {
  if (value == null) return null;
  const cfg = METRIC_LABEL[metric];
  if (!cfg) return `${metric}: ${value}`;
  const v = typeof value === 'number' ? value.toFixed(cfg.fixed) : value;
  return `${cfg.icon} ${v}${cfg.label}`;
}

function getLatestValue(readings, metric) {
  if (!readings) return null;
  const v = readings[metric];
  if (v == null) return null;
  if (Array.isArray(v)) {
    const last = v[v.length - 1];
    return last?.value ?? last ?? null;
  }
  return v?.value ?? v ?? null;
}

export default function SensorNode3D({ config, readings, status }) {
  const temperature = getLatestValue(readings, 'temperature');
  const humidity = getLatestValue(readings, 'humidity');
  const soilTemp = getLatestValue(readings, 'soil_temperature');
  const soilMoisture = getLatestValue(readings, 'soil_moisture');

  const isOnline = status !== 'offline' && status !== 'error';

  const keyMetrics = [
    temperature != null ? formatMetric('temperature', temperature) : null,
    humidity != null ? formatMetric('humidity', humidity) : null,
  ].filter(Boolean);

  const soilMetrics = [
    soilTemp != null ? formatMetric('soil_temperature', soilTemp) : null,
    soilMoisture != null ? formatMetric('soil_moisture', soilMoisture) : null,
  ].filter(Boolean);

  const hasData = keyMetrics.length > 0 || soilMetrics.length > 0;

  return (
    <>
      <div className={`sensor-dot ${isOnline ? 'online' : 'offline'}`} />
      <div className="sensor-label" aria-hidden="true">
        <div className="sensor-label-node">{config.nodeId}</div>
        {!hasData ? (
          <div className="sensor-label-na">no data</div>
        ) : (
          <>
            {keyMetrics.length > 0 && (
              <div className="sensor-label-group">
                <span className="sensor-label-group-title">环境</span>
                {keyMetrics.map((m, i) => (
                  <div key={i} className="sensor-label-line">{m}</div>
                ))}
              </div>
            )}
            {soilMetrics.length > 0 && (
              <div className="sensor-label-group">
                <span className="sensor-label-group-title">土壤</span>
                {soilMetrics.map((m, i) => (
                  <div key={i} className="sensor-label-line">{m}</div>
                ))}
              </div>
            )}
          </>
        )}
      </div>
    </>
  );
}
