import { useEffect, useState, useCallback } from 'react';
import FarmScene from '../components/FarmScene';
import ZoneInfoPanel from '../components/ZoneInfoPanel';
import * as api from '../api';
import { useSSE } from '../hooks/useSSE';
import { farmLayout } from '../config/farmLayout';
import './ZoneDetail.css';

function matchZoneName(areaName) {
  return farmLayout.zones.find(z =>
    z.name === areaName ||
    z.name.endsWith(areaName) ||
    z.name.includes(areaName)
  );
}

function computeZoneStatus(devices, comfortConfig) {
  if (!comfortConfig || !devices || devices.length === 0) return 'normal';
  const latest = {};
  devices.forEach(d => {
    const lr = d.latest_readings || {};
    Object.entries(lr).forEach(([metric, v]) => {
      const ts = v?.timestamp || 0;
      if (!latest[metric] || ts > (latest[metric].timestamp || 0)) {
        latest[metric] = v;
      }
    });
  });
  const getV = (m) => {
    const v = latest[m];
    return v?.value ?? v ?? null;
  };
  const temp = getV('temperature');
  const hum = getV('humidity');
  const light = getV('light');
  const tc = comfortConfig.temperature;
  const hc = comfortConfig.humidity;
  const lc = comfortConfig.light;
  if (tc && temp != null && (temp < tc.min || temp > tc.max)) return 'critical';
  if (hc && hum != null && (hum < hc.min || hum > hc.max)) return 'warning';
  if (lc && light != null && (light < lc.min || light > lc.max)) return 'warning';
  return 'normal';
}

function buildZoneDataMap(areas) {
  const map = {};
  (areas || []).forEach(area => {
    const matched = matchZoneName(area.name);
    if (!matched) return;
    const devices = (area.devices || []).map(d => ({
      ...d,
      readings: d.readings || {},
      latest_readings: (() => {
        const lr = {};
        Object.entries(d.readings || {}).forEach(([metric, pts]) => {
          if (pts?.length > 0) lr[metric] = pts[pts.length - 1];
        });
        return lr;
      })(),
    }));
    map[matched.id] = {
      id: matched.id,
      name: area.name,
      crop_batch: area.crop_batch || null,
      devices,
      status: computeZoneStatus(devices, area.crop_batch?.comfort_config),
    };
  });
  return map;
}

export default function ZoneDetail() {
  const [areas, setAreas] = useState([]);
  const [loading, setLoading] = useState(true);
  const [selectedZoneId, setSelectedZoneId] = useState(null);

  const fetchAll = useCallback(async () => {
    try {
      const [areaData, nodeData] = await Promise.all([
        api.getAreaReadings(),
        api.getDashboardNodeReadings(),
      ]);
      const merged = mergeNodeReadings(areaData, nodeData);
      setAreas(merged?.areas || []);
    } catch (_) {}
    setLoading(false);
  }, []);

  useSSE('/api/v1/events', useCallback((data) => {
    if (data?.type === 'telemetry') fetchAll();
  }, [fetchAll]));

  useEffect(() => {
    fetchAll();
    const interval = setInterval(fetchAll, 10000);
    return () => clearInterval(interval);
  }, [fetchAll]);

  const zoneDataMap = buildZoneDataMap(areas);
  const selectedZoneData = selectedZoneId ? zoneDataMap?.[selectedZoneId] : null;

  if (loading) {
    return (
      <div className="container zone-detail-root">
        <div className="zone-detail-header">
          <h2 tabIndex={-1} id="page-heading">数字孪生</h2>
        </div>
        <div className="scene-container">
          <div className="skeleton" style={{ width: '100%', height: '100%' }} />
        </div>
      </div>
    );
  }

  return (
    <div className="zone-detail-root">
      <div className="zone-detail-header">
        <h2 tabIndex={-1} id="page-heading">基地数字孪生</h2>
      </div>
      <FarmScene
        zoneDataMap={zoneDataMap}
        selectedZoneId={selectedZoneId}
        onZoneClick={setSelectedZoneId}
      />
      {selectedZoneData && (
        <ZoneInfoPanel zoneData={selectedZoneData} onClose={() => setSelectedZoneId(null)} />
      )}
    </div>
  );
}

function mergeNodeReadings(areaRes, nodeRes) {
  if (!areaRes && !nodeRes) return null;
  const areaList = areaRes?.areas || [];
  const nodeList = nodeRes?.areas || [];
  if (nodeList.length === 0) return areaRes;
  return {
    areas: areaList.map(area => {
      const nodeArea = nodeList.find(n => n.area_id === area.id);
      if (!nodeArea) return area;
      return {
        ...area,
        devices: (area.devices || []).map(dev => {
          const node = nodeArea.nodes?.find(n => n.node_id === dev.node_id);
          if (!node) return dev;
          return {
            ...dev,
            readings: {
              ...(dev.readings || {}),
              ...(node.history_24h || {}),
            },
            latest_readings: node.latest || dev.latest_readings || {},
          };
        }),
      };
    }),
  };
}
