import { useEffect, useState } from 'react';
import * as api from '../api';
import { IconAlertTriangle } from '../components/Icons';

export default function Alerts() {
  const [alerts, setAlerts] = useState([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.listAlerts().then(setAlerts).catch(() => {}).finally(() => setLoading(false));
    const interval = setInterval(() => api.listAlerts().then(setAlerts).catch(() => {}), 10000);
    return () => clearInterval(interval);
  }, []);

  if (loading) return <div className="container"><div className="skeleton" style={{ height: 400 }} /></div>;

  return (
    <div className="container">
      <div className="page-header"><h2 tabIndex={-1} id="page-heading"><IconAlertTriangle size={22} style={{verticalAlign:'-3px',marginRight:6}} />告警中心</h2></div>

      <div className="card" style={{ padding: 0 }}>
        <div className="table-wrap">
          <table aria-label="告警记录列表">
            <thead>
              <tr><th>时间</th><th>设备</th><th>命令</th><th>状态</th></tr>
            </thead>
            <tbody>
              {(alerts || []).length === 0 ? (
                <tr><td colSpan={4} style={{ textAlign: 'center', padding: 40 }}><span className="text-dim">暂无告警记录</span></td></tr>
              ) : (
                (alerts || []).map((a, i) => (
                  <tr key={i}>
                    <td className="text-sm">{a.created_at ? new Date(a.created_at * 1000).toLocaleString() : '--'}</td>
                    <td className="text-sm">{a.device_name || a.node_id || '--'}</td>
                    <td className="text-sm">{a.command || '--'}</td>
                    <td>
                      <span className={`badge ${a.status === 'success' ? 'badge-green' : a.status === 'failed' ? 'badge-red' : 'badge-yellow'}`}>
                        {a.status || 'pending'}
                      </span>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
