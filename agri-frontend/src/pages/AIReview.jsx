import { useEffect, useState, useMemo, useCallback } from 'react';
import * as api from '../api';
import { IconLightbulb, IconBarChart3 } from '../components/Icons';

const SEVERITY = {
  failed: { bg: '#451a1a', border: '#EF4444', badge: 'badge-red', label: '危急' },
  success: { bg: '#1a2e1a', border: '#22C55E', badge: 'badge-green', label: '正常' },
  pending: { bg: '#2e2a1a', border: '#F59E0B', badge: 'badge-yellow', label: '注意' },
};

export default function AIReview() {
  const [alerts, setAlerts] = useState([]);
  const [rules, setRules] = useState([]);
  const [devices, setDevices] = useState([]);
  const [summary, setSummary] = useState(null);
  const [loading, setLoading] = useState(true);
  const [period, setPeriod] = useState('7d');

  const fetchAll = useCallback(() => {
    Promise.all([
      api.listAlerts().then(setAlerts).catch(() => {}),
      api.listRules().then(setRules).catch(() => {}),
      api.listDevices().then(setDevices).catch(() => {}),
      api.getDashboardSummary().then(setSummary).catch(() => {}),
    ]).finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    fetchAll();
    const interval = setInterval(fetchAll, 15000);
    return () => clearInterval(interval);
  }, [fetchAll]);

  const diagnoses = useMemo(() => {
    const items = [];

    const failedAlerts = alerts.filter(a => a.status === 'failed');
    if (failedAlerts.length > 0) {
      const latest = failedAlerts[0];
      items.push({
        severity: 'failed',
        title: `${failedAlerts.length} 条命令执行失败`,
        message: `最近一条：设备「${latest.device_name || latest.node_id || '--'}」命令「${latest.command || '--'}」于 ${latest.created_at ? new Date(latest.created_at * 1000).toLocaleString() : '--'} 执行失败，请检查设备状态。`,
        time: latest.created_at ? formatTimeAgo(latest.created_at) : '--',
      });
    }

    const offlineDevices = devices.filter(d => d.status !== 'online');
    if (offlineDevices.length > 0) {
      items.push({
        severity: 'failed',
        title: `${offlineDevices.length} 个设备离线`,
        message: `离线设备：${offlineDevices.map(d => d.name || d.node_id).join('、')}。请检查设备电源和网络连接。`,
        time: '实时',
      });
    }

    const activeRules = rules.filter(r => r.enabled === true || r.enabled === 1);
    if (activeRules.length > 0) {
      items.push({
        severity: 'success',
        title: `${activeRules.length} 条规则运行中`,
        message: `当前活跃规则：${activeRules.map(r => r.name).join('、')}。系统正在自动执行环境调控策略。`,
        time: '实时',
      });
    }

    if (alerts.filter(a => a.status === 'success').length > 5) {
      const recent = alerts.filter(a => a.status === 'success').slice(0, 3);
      items.push({
        severity: 'success',
        title: `最近成功执行 ${recent.length} 条命令`,
        message: recent.map(a => `设备「${a.device_name || a.node_id || '--'}」→ ${a.command || '--'}`).join('；'),
        time: '实时',
      });
    }

    if (items.length === 0) {
      items.push({
        severity: 'success',
        title: '系统运行正常',
        message: '未检测到异常告警，所有规则和命令执行均正常。',
        time: '实时',
      });
    }

    return items;
  }, [alerts, rules, devices]);

  const recentAlerts = useMemo(() => {
    const cutoff = period === 'all' ? 0
      : period === '30d' ? Math.floor(Date.now() / 1000) - 86400 * 30
      : period === '7d' ? Math.floor(Date.now() / 1000) - 86400 * 7
      : Math.floor(Date.now() / 1000) - 86400;
    const filtered = cutoff ? alerts.filter(a => (a.created_at || 0) >= cutoff) : alerts;
    return filtered.slice(0, 20);
  }, [alerts, period]);

  if (loading) {
    return (
      <div className="container">
      <div className="page-header"><h2 tabIndex={-1} id="page-heading">AI 决策与复盘</h2></div>
        <div className="skeleton" style={{ height: 200 }} />
        <div className="skeleton" style={{ height: 300, marginTop: 16 }} />
      </div>
    );
  }

  return (
    <div className="container">
      <div className="page-header"><h2 tabIndex={-1} id="page-heading">AI 决策与复盘</h2></div>

      <div className="grid grid-3 mb-lg">
        <div className="card" style={{ textAlign: 'center' }}>
          <div className="text-2xl fw-700" style={{ color: 'var(--red)' }}>{alerts.filter(a => a.status === 'failed').length}</div>
          <div className="text-xs text-dim mt-xs">执行失败</div>
        </div>
        <div className="card" style={{ textAlign: 'center' }}>
          <div className="text-2xl fw-700" style={{ color: 'var(--green)' }}>{alerts.filter(a => a.status === 'success').length}</div>
          <div className="text-xs text-dim mt-xs">执行成功</div>
        </div>
        <div className="card" style={{ textAlign: 'center' }}>
          <div className="text-2xl fw-700" style={{ color: 'var(--yellow)' }}>{alerts.filter(a => a.status === 'pending').length}</div>
          <div className="text-xs text-dim mt-xs">待处理</div>
        </div>
      </div>

      <div className="grid grid-2" style={{ gridTemplateColumns: '3fr 2fr' }}>
        <div>
          <div className="flex items-center gap-sm mb-md">
            <IconLightbulb size={18} />
            <span className="fw-600 text-sm">智能诊断报告</span>
          </div>

          {diagnoses.map((d, i) => {
            const s = SEVERITY[d.severity] || SEVERITY.pending;
            return (
              <div key={i} className="card mb-md" style={{ background: s.bg, borderLeft: `4px solid ${s.border}` }}>
                <div className="flex justify-between items-center mb-sm">
                  <div className="flex items-center gap-sm">
                    <span className={`badge ${s.badge}`}>{s.label}</span>
                    <span className="fw-600 text-sm text-bright">{d.title}</span>
                  </div>
                  <span className="text-xs text-dim">{d.time}</span>
                </div>
                <p className="text-sm text-dim mb-md">{d.message}</p>
              </div>
            );
          })}
        </div>

        <div>
          <div className="flex items-center gap-sm mb-md">
            <IconBarChart3 size={18} />
            <span className="fw-600 text-sm">历史复盘</span>
          </div>

          <div className="card">
            <div className="text-sm text-dim mb-md">选择对比周期</div>
            <div className="grid grid-2 mb-md">
              {[
                { key: '1d', label: '近 24 小时' },
                { key: '7d', label: '近 7 天' },
                { key: '30d', label: '近 30 天' },
                { key: 'all', label: '全部' },
              ].map(p => (
                <button
                  key={p.key}
                  className={`btn btn-sm ${period === p.key ? 'btn-active' : ''}`}
                  style={{ width: '100%', justifyContent: 'center' }}
                  onClick={() => setPeriod(p.key)}
                >
                  {p.label}
                </button>
              ))}
            </div>

            <div className="separator" />
            <div className="text-sm fw-500 mt-md mb-sm">命令执行记录</div>
            <div style={{ maxHeight: 360, overflowY: 'auto' }}>
              {recentAlerts.length === 0 ? (
                <div className="text-xs text-dim" style={{ textAlign: 'center', padding: 20 }}>暂无记录</div>
              ) : (
                recentAlerts.map((a, i) => (
                  <div key={i} className="flex justify-between items-center py-xs" style={{ borderBottom: '1px solid var(--border)' }}>
                    <div>
                      <div className="text-xs text-bright">{a.device_name || a.node_id || '--'}</div>
                      <div className="text-xs text-dim">{a.command || '--'}</div>
                    </div>
                    <div className="flex items-center gap-sm">
                      <span className={`badge ${a.status === 'success' ? 'badge-green' : a.status === 'failed' ? 'badge-red' : 'badge-yellow'}`} style={{ fontSize: 10 }}>
                        {a.status || 'pending'}
                      </span>
                      <span className="text-xs text-dim">{a.created_at ? new Date(a.created_at * 1000).toLocaleString() : '--'}</span>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function formatTimeAgo(ts) {
  const diff = Math.floor(Date.now() / 1000) - ts;
  if (diff < 60) return '刚刚';
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`;
  return `${Math.floor(diff / 86400)} 天前`;
}
