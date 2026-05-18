const METRIC_LIMITS = {
  temperature:      { min: -15, max: 60,   maxRate: 0.5  },
  humidity:         { min: 0,   max: 100,  maxRate: 2    },
  soil_moisture:    { min: 0,   max: 100,  maxRate: 2    },
  soil_temperature: { min: -15, max: 55,   maxRate: 0.5  },
  light:            { min: 0,   max: 2e5,  maxRate: 1000 },
  ec:               { min: 0,   max: 10,   maxRate: 0.2  },
};

const STALE_TTL = 1800;

const state = {};

function key(deviceId, metric) {
  return `${deviceId}::${metric}`;
}

function getTracked(deviceId, metric) {
  const k = key(deviceId, metric);
  if (!state[k]) {
    state[k] = { lastVal: null, lastTs: null };
  } else if (state[k].lastTs !== null) {
    const now = Math.floor(Date.now() / 1000);
    if (now - state[k].lastTs > STALE_TTL) {
      state[k] = { lastVal: null, lastTs: null };
    }
  }
  return state[k];
}

function median(values) {
  if (values.length === 0) return 0;
  const s = [...values].sort((a, b) => a - b);
  const m = Math.floor(s.length / 2);
  return s.length % 2 === 0 ? (s[m - 1] + s[m]) / 2 : s[m];
}

export function filterReadings(deviceId, metric, readings) {
  if (!readings || readings.length === 0) return [];
  const limits = METRIC_LIMITS[metric];
  if (!limits) return readings;

  const t = getTracked(deviceId, metric);

  const passed = [];
  for (const pt of readings) {
    if (pt.value == null) continue;

    if (pt.value < limits.min || pt.value > limits.max) continue;

    // Rate check only for genuinely new readings (timestamp > lastTs).
    // Re-processing old history data on each fetch skips rate check.
    if (t.lastVal !== null && t.lastTs !== null && pt.timestamp > t.lastTs) {
      const dt = Math.max(0.001, pt.timestamp - t.lastTs);
      const rate = Math.abs(pt.value - t.lastVal) / dt;
      if (rate > limits.maxRate) continue;
    }

    // Track the newest accepted reading only
    if (t.lastTs === null || pt.timestamp > t.lastTs) {
      t.lastVal = pt.value;
      t.lastTs = pt.timestamp;
    }

    passed.push(pt);
  }

  if (passed.length > 1) {
    const ws = Math.min(5, passed.length);
    const originalValues = passed.map(p => p.value);
    for (let i = 0; i < passed.length; i++) {
      const start = Math.max(0, i - ws + 1);
      const windowVals = originalValues.slice(start, i + 1);
      passed[i] = { ...passed[i], value: median(windowVals) };
    }
  }

  return passed;
}

export function filterLatestReading(deviceId, metric, reading) {
  if (!reading || reading.value == null) return null;
  const limits = METRIC_LIMITS[metric];
  if (!limits) return reading;

  const { min, max } = limits;
  if (reading.value < min || reading.value > max) return null;

  const t = getTracked(deviceId, metric);

  if (t.lastVal !== null && t.lastTs !== null && reading.timestamp > t.lastTs) {
    const dt = Math.max(0.001, reading.timestamp - t.lastTs);
    const rate = Math.abs(reading.value - t.lastVal) / dt;
    if (rate > limits.maxRate) return null;
  }

  if (t.lastTs === null || reading.timestamp > t.lastTs) {
    t.lastVal = reading.value;
    t.lastTs = reading.timestamp;
  }

  return reading;
}

export function filterNodeData(apiResponse) {
  if (!apiResponse?.areas) return apiResponse;
  return {
    ...apiResponse,
    areas: apiResponse.areas.map(area => ({
      ...area,
      nodes: (area.nodes || []).map(node => ({
        ...node,
        history_24h: Object.fromEntries(
          Object.entries(node.history_24h || {}).map(([metric, readings]) => [
            metric,
            filterReadings(node.node_id, metric, readings),
          ])
        ),
        latest: Object.fromEntries(
          Object.entries(node.latest || {})
            .map(([metric, reading]) => [metric, filterLatestReading(node.node_id, metric, reading)])
            .filter(([, v]) => v != null)
        ),
      })),
      devices: (area.devices || []).map(dev => ({
        ...dev,
        readings: Object.fromEntries(
          Object.entries(dev.readings || {}).map(([metric, readings]) => [
            metric,
            filterReadings(dev.node_id || dev.id, metric, readings),
          ])
        ),
      })),
    })),
  };
}
