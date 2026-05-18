const BASE = '/api/v1';

async function fetchJSON(url) {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export function listDevices() { return fetchJSON(`${BASE}/devices`); }
export function getDevice(id) { return fetchJSON(`${BASE}/devices/${id}`); }
export function listReadings(deviceId, metric, limit = 100) {
  return fetchJSON(`${BASE}/devices/${deviceId}/readings?metric=${metric}&limit=${limit}`);
}
export function listRules() { return fetchJSON(`${BASE}/rules`); }
export function listAlerts() { return fetchJSON(`${BASE}/alerts`); }
export function getDashboardSummary() { return fetchJSON(`${BASE}/dashboard/summary`); }
export function getAreaReadings() { return fetchJSON(`${BASE}/dashboard/area-readings`); }
export function getSystemInfo() { return fetchJSON(`${BASE}/system/info`); }
export function listAreas() { return fetchJSON(`${BASE}/areas`); }
export function listCrops() { return fetchJSON(`${BASE}/crops`); }
export function getDashboardNodeReadings() { return fetchJSON(`${BASE}/dashboard/node-readings`); }
export function sendCommand(deviceId, command, params = {}) {
  return fetch(`${BASE}/devices/${deviceId}/command`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({ command, params }),
  }).then(r => r.json());
}
export function updateArea(id, data) {
  return fetch(`${BASE}/areas/${id}`, { method: 'PUT', headers: {'Content-Type':'application/json'}, body: JSON.stringify(data) }).then(r => r.json());
}
export function updateCropName(areaId, cropName) {
  return fetch(`${BASE}/areas/${areaId}/crop-name`, { method: 'PUT', headers: {'Content-Type':'application/json'}, body: JSON.stringify({ crop_name: cropName }) }).then(r => r.json());
}

// Weather API
export function getWeatherNow(loc) { return fetchJSON(`${BASE}/weather/now?location=${loc}`); }
export function getWeather3d(loc) { return fetchJSON(`${BASE}/weather/3d?location=${loc}`); }
export function getWeather24h(loc) { return fetchJSON(`${BASE}/weather/24h?location=${loc}`); }
export function getWeatherMinutely(loc) { return fetchJSON(`${BASE}/weather/minutely?location=${loc}`); }
export function getWeatherAir(loc) { return fetchJSON(`${BASE}/weather/air?location=${loc}`); }
export function getWeatherIndices(loc, types) { return fetchJSON(`${BASE}/weather/indices?location=${loc}&type=${types}`); }
export function getWeatherWarning(loc) { return fetchJSON(`${BASE}/weather/warning?location=${loc}`); }
export function searchLocation(q) { return fetchJSON(`${BASE}/weather/geo?location=${encodeURIComponent(q)}`); }
