import { IconX, IconSprout, IconThermometer, IconDroplets } from './Icons';

const STATUS_MAP = {
  critical: { label: '故障', color: 'var(--red)', cls: 'badge-red' },
  warning: { label: '预警', color: 'var(--yellow)', cls: 'badge-yellow' },
  normal: { label: '正常', color: 'var(--green)', cls: 'badge-green' },
};

export default function ZoneInfoPanel({ zoneData, onClose }) {
  if (!zoneData) return null;

  const s = STATUS_MAP[zoneData.status] || STATUS_MAP.normal;
  const devices = zoneData.devices || [];
  const crop = zoneData.crop_batch;
  const comfort = crop?.comfort_config || {};

  const getLatest = (device, metric) => {
    const v = device.latest_readings?.[metric];
    if (v == null) return null;
    return v?.value ?? v;
  };

  return (
    <div className="zone-info-panel">
      <div className="zone-info-header">
        <div>
          <span className="fw-600 text-sm">{zoneData.name}</span>
          <span className={`badge ${s.cls}`} style={{ marginLeft: 8 }}>{s.label}</span>
        </div>
        <button className="btn-icon" onClick={onClose} aria-label="关闭面板">
          <IconX size={16} />
        </button>
      </div>

      {crop && (
        <div className="zone-info-section">
          <div className="zone-info-section-title"><IconSprout size={14} style={{verticalAlign:-2,marginRight:4}} />种植信息</div>
          <div className="zone-info-row"><span className="text-dim">作物</span><span>{crop.crop_name || '--'}</span></div>
          {crop.plant_date > 0 && (
            <div className="zone-info-row"><span className="text-dim">定植</span><span>{new Date(crop.plant_date * 1000).toLocaleDateString()}</span></div>
          )}
        </div>
      )}

      {Object.keys(comfort).length > 0 && (
        <div className="zone-info-section">
          <div className="zone-info-section-title"><IconThermometer size={14} style={{verticalAlign:-2,marginRight:4}} />舒适区间</div>
          {comfort.temperature && (
            <div className="zone-info-row"><span className="text-dim">温度</span><span>{comfort.temperature.min}~{comfort.temperature.max}°C</span></div>
          )}
          {comfort.humidity && (
            <div className="zone-info-row"><span className="text-dim">湿度</span><span>{comfort.humidity.min}~{comfort.humidity.max}%</span></div>
          )}
          {comfort.light && (
            <div className="zone-info-row"><span className="text-dim">光照</span><span>{comfort.light.min}~{comfort.light.max} lux</span></div>
          )}
        </div>
      )}

      <div className="zone-info-section">
        <div className="zone-info-section-title"><IconDroplets size={14} style={{verticalAlign:-2,marginRight:4}} />设备 ({devices.length})</div>
        {devices.length === 0 ? (
          <div className="text-dim text-xs">暂无设备</div>
        ) : (
          devices.map(dev => {
            const t = getLatest(dev, 'temperature');
            const h = getLatest(dev, 'humidity');
            return (
              <div key={dev.id || dev.node_id} className="zone-info-device">
                <div className="zone-info-row">
                  <span className="text-dim">{dev.name || dev.node_id}</span>
                  <span className={`text-xs ${dev.status === 'offline' ? 'text-dim' : ''}`}>
                    {dev.status || 'online'}
                  </span>
                </div>
                <div className="zone-info-metrics">
                  {t != null && <span><IconThermometer size={10} style={{verticalAlign:-1}} /> {t}°C</span>}
                  {h != null && <span><IconDroplets size={10} style={{verticalAlign:-1}} /> {h}%</span>}
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
