import { useEffect, useState, useCallback, useRef } from 'react';
import { useStore } from '../store';
import * as api from '../api';
import WeatherCard from '../components/WeatherCard';
import { useSSE } from '../hooks/useSSE';
import { navigate } from '../App';
import { AreaChart, Area, XAxis, YAxis, Tooltip, ResponsiveContainer, ReferenceArea } from 'recharts';
import { filterNodeData } from '../utils/sensorFilter';
import { IconMapPin, IconCircle, IconThermometer, IconDroplets, IconLeaf, IconZap, IconSun, IconSprout, IconCheck } from '../components/Icons';

const NODE_COLORS = ['#0EA5E9', '#F59E0B', '#22C55E', '#A855F7', '#EF4444', '#06B6D4'];

function downsample(data, maxPoints = 200) {
  if (!data || data.length <= maxPoints) return data;
  const step = Math.ceil(data.length / maxPoints);
  const sampled = data.filter((_, i) => i % step === 0);
  if (sampled[sampled.length - 1] !== data[data.length - 1]) {
    sampled.push(data[data.length - 1]);
  }
  return sampled;
}

function buildChartData(nodes, metric) {
  const dataMap = {};
  nodes.forEach((node, idx) => {
    const col = `n${idx + 1}`;
    const raw = node.history_24h[metric] || node.device_readings?.[metric] || [];
    const sampled = downsample(raw);
    sampled.forEach(pt => {
      const t = pt.timestamp;
      if (!dataMap[t]) dataMap[t] = { time: t };
      dataMap[t][col] = pt.value;
    });
  });
  return Object.values(dataMap).sort((a, b) => a.time - b.time);
}

function pickMetric(nodes, preferred) {
  const allMetrics = ['temperature', 'humidity', 'soil_moisture', 'soil_temperature', 'light', 'ec'];
  for (const m of [...preferred, ...allMetrics]) {
    if (nodes.some(n => n.history_24h?.[m]?.length > 0 || n.device_readings?.[m]?.length > 0)) return m;
  }
  return preferred[0];
}



function MiniChart({ data, lines, comfortRange, label, yDomain }) {
  const chartRef = useRef(null);
  const dataLenRef = useRef(data.length);
  dataLenRef.current = data.length;
  const userZoomed = useRef(false);

  const [zoom, setZoom] = useState({ startPct: 0, endPct: 1 });

  const prevHasData = useRef(false);
  useEffect(() => {
    const hasData = data.length > 0;
    if (hasData && !prevHasData.current) {
      setZoom({ startPct: 0, endPct: 1 });
      userZoomed.current = false;
    }
    prevHasData.current = hasData;
  }, [data.length]);

  useEffect(() => {
    const el = chartRef.current;
    if (!el) return;
    const handler = (e) => {
      e.preventDefault();
      const rect = el.getBoundingClientRect();
      const leftPad = 50;
      const rightPad = 10;
      const plotW = rect.width - leftPad - rightPad;
      const ratio = plotW > 0 ? Math.max(0, Math.min(1, (e.clientX - rect.left - leftPad) / plotW)) : 0.5;
      userZoomed.current = true;
      const dy = e.deltaY;
      setZoom(prev => {
        const range = prev.endPct - prev.startPct;
        if (range <= 0.01) return prev;

        const factor = dy > 0 ? 1.2 : 0.8;
        const newRange = Math.min(1, Math.max(0.02, range * factor));
        const mousePct = prev.startPct + ratio * range;
        let startPct = mousePct - ratio * newRange;
        let endPct = startPct + newRange;
        if (startPct < 0) { startPct = 0; endPct = newRange; }
        if (endPct > 1) { endPct = 1; startPct = 1 - newRange; }
        return { startPct, endPct };
      });
    };
    el.addEventListener('wheel', handler, { passive: false });
    return () => el.removeEventListener('wheel', handler);
  }, [data.length]);

  const startIdx = Math.round(zoom.startPct * (data.length - 1));
  const endIdx = Math.round(zoom.endPct * (data.length - 1));
  const visibleData = data.slice(startIdx, endIdx + 1);

  const maxPoints = 300;
  const displayData = visibleData.length > maxPoints
    ? downsample(visibleData, maxPoints)
    : visibleData;

  const fmtX = (ts) => {
    if (!ts) return '';
    const d = new Date(ts * 1000);
    return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
  };

  const fmtY = (v) => {
    if (v == null || !isFinite(v)) return '';
    return v % 1 === 0 ? v.toString() : v.toFixed(1);
  };

  if (!displayData || displayData.length === 0) {
    return (
      <div className="mini-chart">
        <div className="mini-chart-label">{label}</div>
        <div className="mini-chart-na">暂无数据</div>
      </div>
    );
  }

  return (
    <div className="mini-chart" ref={chartRef} tabIndex={0}>
      <div className="mini-chart-label">{label}</div>
      <ResponsiveContainer width="100%" height={150}>
        <AreaChart data={displayData} margin={{ top: 4, right: 4, bottom: 0, left: 4 }}>
          {comfortRange && comfortRange.min != null && comfortRange.max != null && (
            <ReferenceArea y1={comfortRange.min} y2={comfortRange.max} fill="rgba(64,192,87,0.1)" />
          )}
          <XAxis
            dataKey="time"
            tickFormatter={fmtX}
            tick={{ fontSize: 10, fill: 'var(--text-dim)' }}
            tickLine={false}
            axisLine={{ stroke: 'var(--border)' }}
            minTickGap={50}
          />
          <YAxis
            tickFormatter={fmtY}
            tick={{ fontSize: 10, fill: 'var(--text-dim)' }}
            tickLine={false}
            axisLine={{ stroke: 'var(--border)' }}
            width={40}
            domain={yDomain || ['auto', 'auto']}
            tickCount={5}
          />
          <Tooltip
            contentStyle={{ background: 'var(--bg-card)', border: '1px solid var(--border)', borderRadius: 6, fontSize: 11, padding: '4px 8px' }}
            labelFormatter={fmtX}
          />
          {lines.map(l => (
            <Area key={l.dataKey} type="monotone" dataKey={l.dataKey} stroke={l.color} fill={l.color} fillOpacity={0.05} strokeWidth={1.5} dot={false} name={l.name} />
          ))}
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}

export default function Overview() {
  const { summary, setSummary, setSelectedZone } = useStore();
  const [areas, setAreas] = useState([]);
  const [nodeData, setNodeData] = useState([]);
  const [loading, setLoading] = useState(true);
  const [editingCrop, setEditingCrop] = useState(null);
  const [cropInput, setCropInput] = useState({});
  const [chartModes, setChartModes] = useState({});

  const fetchAll = useCallback(async () => {
    const [s, a, n] = await Promise.all([
      api.getDashboardSummary().catch(() => null),
      api.getAreaReadings().catch(() => null),
      api.getDashboardNodeReadings().catch(() => null),
    ]);
    if (s) setSummary(s);
    if (a) {
      const cleaned = filterNodeData(a);
      const list = cleaned.areas || [];
      setAreas(list);
      setCropInput(prev => {
        const next = { ...prev };
        list.forEach(a2 => { if (!next[a2.id]) next[a2.id] = a2.crop_batch?.crop_name || '未命名'; });
        return next;
      });
    }
    if (n) {
      const cleaned = filterNodeData(n);
      setNodeData(cleaned.areas || []);
    }
  }, [setSummary]);

  const live = useSSE('/api/v1/events', useCallback((data) => {
    if (data?.type === 'telemetry') fetchAll();
  }, [fetchAll]));

  useEffect(() => {
    fetchAll().finally(() => setLoading(false));
    const interval = setInterval(fetchAll, 15000);
    return () => clearInterval(interval);
  }, [fetchAll]);

  const saveCropName = (areaId) => {
    const name = cropInput[areaId] || '未命名';
    setEditingCrop(null);
    setAreas(prev => prev.map(a =>
      a.id === areaId ? { ...a, crop_batch: { ...a.crop_batch, crop_name: name } } : a
    ));
    api.updateCropName(areaId, name).catch(() => {});
  };

  const setChartMode = (areaId, mode) => {
    setChartModes(prev => ({ ...prev, [areaId]: mode }));
  };

  if (loading) {
    return (
      <div className="container">
        <div className="page-header">
          <div className="page-header-row">
            <h2>全局态势感知</h2>
            <span className="live-badge"><span className="pulse-dot" />连接中...</span>
          </div>
        </div>
        <div className="skeleton" style={{ height: 160, marginBottom: 16 }} />
        <div className="skeleton" style={{ height: 100 }} />
        <div className="skeleton" style={{ height: 100, marginTop: 12 }} />
      </div>
    );
  }

  return (
    <div className="container">
      <div className="page-header">
        <div className="page-header-row">
          <h2 tabIndex={-1} id="page-heading">全局态势感知</h2>
          <span className={`live-badge ${live ? 'live' : ''}`}>
            <span className="pulse-dot" />{live ? '实时' : '连接中...'}
          </span>
        </div>
        <div className="sr-only" role="status" aria-live="polite" aria-atomic="true">
          已更新 {areas.length} 个区域数据
        </div>
        <div className="kpi-area">
          <div className="kpi-grid">
            <div className="card" style={{ textAlign: 'center', padding: '14px 12px' }}>
              <div className="text-xs text-dim mb-xs"><IconSprout size={14} style={{verticalAlign:'-2px',marginRight:4}} />设备总数</div>
              <div className="text-xl fw-700 text-bright">{summary?.total_devices ?? 0}</div>
            </div>
            <div className="card" style={{ textAlign: 'center', padding: '14px 12px' }}>
              <div className="text-xs text-dim mb-xs"><IconZap size={14} style={{verticalAlign:'-2px',marginRight:4}} />在线设备</div>
              <div className="text-xl fw-700" style={{ color: 'var(--green)' }}>{summary?.online_devices ?? 0}</div>
            </div>
            <div className="card" style={{ textAlign: 'center', padding: '14px 12px' }}>
              <div className="text-xs text-dim mb-xs"><IconCheck size={14} style={{verticalAlign:'-2px',marginRight:4}} />活跃规则</div>
              <div className="text-xl fw-700" style={{ color: 'var(--blue)' }}>{summary?.active_rules ?? 0}</div>
            </div>
            <div className="card" style={{ textAlign: 'center', padding: '14px 12px' }}>
              <div className="text-xs text-dim mb-xs"><IconMapPin size={14} style={{verticalAlign:'-2px',marginRight:4}} />区域数量</div>
              <div className="text-xl fw-700 text-bright">{areas.length}</div>
            </div>
          </div>
          <WeatherCard />
        </div>
      </div>

      <div className="flex items-center gap-sm mb-md">
        <IconMapPin size={18} />
        <span className="fw-600 text-sm">区域状态概览</span>
      </div>

      {areas.length === 0 ? (
        <div className="card" style={{ textAlign: 'center', padding: 40 }}>
          <span className="text-dim">暂无区域数据，请先在系统设置中添加区域和设备</span>
        </div>
      ) : (
        areas.map(area => {
          const comfort = area.crop_batch?.comfort_config || {};
          const nd = nodeData.find(n => n.area_id === area.id);
          const areaDevices = area.devices || [];

          // Merge node data (from node-readings) with device readings (from area-readings)
          // to fill metrics missing from node-readings (e.g., light)
          const nodes = (nd?.nodes || []).map(node => {
            const devReadings = {};
            areaDevices.forEach(dev => {
              if (dev.node_id === node.node_id || dev.id === node.node_id) {
                Object.entries(dev.readings || {}).forEach(([metric, pts]) => {
                  if (!devReadings[metric] || pts.length > devReadings[metric].length) {
                    devReadings[metric] = pts;
                  }
                });
              }
            });
            return { ...node, device_readings: devReadings };
          });

          // If nodeData didn't have any nodes for this area, create synthetic ones from area devices
          if (nodes.length === 0 && areaDevices.length > 0) {
            areaDevices.forEach((dev, idx) => {
              const latest = {};
              const history_24h = {};
              Object.entries(dev.readings || {}).forEach(([metric, pts]) => {
                history_24h[metric] = pts;
              });
              nodes.push({
                node_id: dev.node_id || dev.id,
                node_number: idx + 1,
                latest,
                history_24h,
                device_readings: dev.readings || {},
              });
            });
          }

          const mode = chartModes[area.id] || 'ambient';

          const latestPerMetric = {};
          nodes.forEach(node => {
            // Prefer node.latest, fallback to last reading from device_readings
            const src = node.latest || {};
            Object.entries(src).forEach(([metric, data]) => {
              if (!latestPerMetric[metric] || data.timestamp > latestPerMetric[metric].timestamp) {
                latestPerMetric[metric] = data;
              }
            });
            // Also check device_readings for metrics not in latest
            Object.entries(node.device_readings || {}).forEach(([metric, pts]) => {
              if (!latestPerMetric[metric] && pts.length > 0) {
                const last = pts[pts.length - 1];
                latestPerMetric[metric] = { value: last.value, timestamp: last.timestamp, unit: last.unit };
              }
            });
          });
          const temp = latestPerMetric.temperature?.value;
          const hum = latestPerMetric.humidity?.value;
          const light = latestPerMetric.light?.value;
          const tc = comfort.temperature;
          const hc = comfort.humidity;
          const lc = comfort.light;
          let status = 'normal', statusLabel = '正常', statusColor = 'var(--green)';
          if (tc && temp != null && (temp < tc.min || temp > tc.max)) { status = 'critical'; statusLabel = '危急'; statusColor = 'var(--red)'; }
          else if (hc && hum != null && (hum < hc.min || hum > hc.max)) { status = 'warning'; statusLabel = '告警'; statusColor = 'var(--yellow)'; }
          else if (lc && light != null && (light < lc.min || light > lc.max)) { status = 'warning'; statusLabel = '告警'; statusColor = 'var(--yellow)'; }

          const chart1Metric = mode === 'ambient' ? 'temperature' : pickMetric(nodes, ['soil_moisture', 'soil_temperature']);
          const chart2Metric = mode === 'ambient' ? 'humidity' : pickMetric(nodes, ['soil_moisture', 'soil_temperature']);
          const chart3Metric = pickMetric(nodes, ['light', 'ec', 'soil_moisture', 'soil_temperature']);
          const chart1Data = buildChartData(nodes, chart1Metric);
          const chart2Data = buildChartData(nodes, chart2Metric);
          const chart3Data = buildChartData(nodes, chart3Metric);

          const chartLines = nodes.map((n, i) => ({ dataKey: `n${i + 1}`, color: NODE_COLORS[i % NODE_COLORS.length], name: `节点${i + 1}` }));

          const comfort1 = comfort[chart1Metric];
          const comfort2 = comfort[chart2Metric];
          const comfort3 = comfort[chart3Metric];

          const yDomainForMetric = { temperature: [0, 50], soil_temperature: [-10, 50], humidity: [0, 100], soil_moisture: [0, 100], light: [0, 200000], ec: [0, 5] };
          const chart1YDomain = yDomainForMetric[chart1Metric];
          const chart2YDomain = yDomainForMetric[chart2Metric];
          const chart3YDomain = yDomainForMetric[chart3Metric];
          const chart3Label = { light: '光照', ec: 'EC', humidity: '湿度', soil_moisture: '土壤湿度', soil_temperature: '土壤温度' }[chart3Metric] || chart3Metric;

          const navigateToZone = () => {
            setSelectedZone(area);
            navigate('/zone');
          };

          return (
            <div
              key={area.id}
              className="card area-card"
              style={{ borderLeft: `4px solid ${statusColor}`, cursor: 'pointer', marginBottom: 12 }}
              onClick={navigateToZone}
              onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); navigateToZone(); } }}
              role="button"
              tabIndex={0}
              aria-label={`区域 ${area.name}，状态 ${statusLabel}，${areaDevices.length} 个设备`}
            >
              <div className="area-card-horiz">
                <div className="area-card-left">
                  <div className="flex items-center gap-xs mb-sm" style={{ flexWrap: 'wrap' }}>
                    <IconCircle size={14} fill={statusColor} style={{ flexShrink: 0 }} />
                    <span className="fw-600 text-sm text-bright">{area.name}</span>
                    {editingCrop === area.id ? (
                      <input className="input area-crop-input" value={cropInput[area.id] || ''}
                        onChange={e => setCropInput(prev => ({...prev, [area.id]: e.target.value}))}
                        onBlur={() => saveCropName(area.id)}
                        onKeyDown={e => e.key === 'Enter' && saveCropName(area.id)}
                        autoFocus onClick={e => e.stopPropagation()} />
                    ) : (
                      <button className="area-crop-name" onClick={e => { e.stopPropagation(); setEditingCrop(area.id); }} aria-label={`修改作物名称，当前：${cropInput[area.id] || '未命名'}`}>
                        {cropInput[area.id] || '未命名'} ✎
                      </button>
                    )}
                    <span className={`badge ${status === 'normal' ? 'badge-green' : status === 'warning' ? 'badge-yellow' : 'badge-red'}`} style={{ marginLeft: 4 }}>
                      {statusLabel}
                    </span>
                  </div>

                  {nodes.length === 0 ? (
                    <div className="text-dim text-sm">暂无节点数据</div>
                  ) : (
                    nodes.map(node => {
                      const nl = node.latest || {};
                      const devR = node.device_readings || {};
                      const getVal = (metric) => {
                        const v = nl[metric]?.value;
                        if (v != null) return v;
                        const pts = devR[metric];
                        if (pts && pts.length > 0) return pts[pts.length - 1].value;
                        return null;
                      };
                      const t = getVal('temperature');
                      const h = getVal('humidity');
                      const st = getVal('soil_temperature');
                      const sm = getVal('soil_moisture');
                      const ec = getVal('ec');
                      return (
                        <div key={node.node_id} className="node-row" onMouseLeave={() => setChartMode(area.id, 'ambient')}>
                          <span className="node-label">节点{node.node_number}:</span>
                          <span className="ambient-group" onMouseEnter={() => setChartMode(area.id, 'ambient')}>
                            <span className="node-group-label">环境</span>
                            <span className="node-metric" title="环境温度"><IconThermometer size={12} style={{verticalAlign:'-2px'}} />{t != null ? `${t}°C` : '--'}</span>
                            <span className="nm-sep">·</span>
                            <span className="node-metric" title="环境湿度"><IconDroplets size={12} style={{verticalAlign:'-2px'}} />{h != null ? `${h}%` : '--'}</span>
                          </span>
                          <span className="nm-sep" style={{ margin: '0 6px', color: 'var(--border)' }}>｜</span>
                          <span className="soil-group" onMouseEnter={() => setChartMode(area.id, 'soil')}>
                            <span className="node-group-label">土壤</span>
                            <span className="node-metric" title="土壤温度"><IconLeaf size={12} style={{verticalAlign:'-2px'}} />{st != null ? `${st}°C` : '--'}</span>
                            <span className="nm-sep">·</span>
                            <span className="node-metric" title="土壤湿度"><IconDroplets size={12} style={{verticalAlign:'-2px'}} />{sm != null ? `${sm}%` : '--'}</span>
                          </span>
                          <span className="nm-sep">·</span>
                          <span className="node-metric" title="EC值"><IconZap size={12} style={{verticalAlign:'-2px'}} />{ec != null ? `${ec}mS/cm` : '--'}</span>
                        </div>
                      );
                    })
                  )}
                </div>

                <div className="area-card-charts" onClick={e => e.stopPropagation()}>
                  <MiniChart data={chart1Data} lines={chartLines} comfortRange={comfort1}
                    label={mode === 'ambient' ? '环境温度' : '土壤温度'}
                    yDomain={chart1YDomain} />
                  <MiniChart data={chart2Data} lines={chartLines} comfortRange={comfort2}
                    label={mode === 'ambient' ? '环境湿度' : '土壤湿度'}
                    yDomain={chart2YDomain} />
                  <MiniChart data={chart3Data} lines={chartLines} comfortRange={comfort3}
                    label={chart3Label}
                    yDomain={chart3YDomain} />
                </div>
              </div>
            </div>
          );
        })
      )}
    </div>
  );
}
