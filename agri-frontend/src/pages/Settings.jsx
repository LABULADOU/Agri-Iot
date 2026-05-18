import { useEffect, useState, useCallback } from 'react';
import * as api from '../api';
import { IconSettings } from '../components/Icons';

export default function Settings() {
  const [info, setInfo] = useState(null);
  const [loading, setLoading] = useState(true);

  const fetchInfo = useCallback(() => {
    api.getSystemInfo().then(setInfo).catch(() => {}).finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    fetchInfo();
    const interval = setInterval(fetchInfo, 30000);
    return () => clearInterval(interval);
  }, [fetchInfo]);

  if (loading) return <div className="container"><div className="skeleton" style={{ height: 400 }} /></div>;

  const s = info?.stats || {};
  const rows = [
    { label: '服务信息', items: [
      ['服务版本', info?.version || '0.2.0'],
      ['服务器时间', info?.server_time ? new Date(info.server_time * 1000).toLocaleString() : '--'],
      ['MQTT Broker', 'broker.emqx.io'],
    ]},
    { label: '数据统计', items: [
      ['设备总数', s.total_devices ?? 0],
      ['在线设备', s.online_devices ?? 0],
      ['规则数量', s.total_rules ?? 0],
      ['活跃规则', s.active_rules ?? 0],
      ['传感器读数', s.total_readings ?? 0],
      ['告警记录', s.total_alerts ?? 0],
    ]},
  ];

  return (
    <div className="container">
      <div className="page-header"><h2 tabIndex={-1} id="page-heading"><IconSettings size={22} style={{verticalAlign:'-3px',marginRight:6}} />系统设置</h2></div>

      <div className="grid grid-2">
        {rows.map(section => (
          <div key={section.label} className="card">
            <div className="card-title">{section.label}</div>
            <div className="grid grid-2" style={{ gap: 12 }}>
              {section.items.map(([label, value]) => (
                <div key={label}>
                  <div className="text-xs text-dim">{label}</div>
                  <div className="text-sm fw-500">{value}</div>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
