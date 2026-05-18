import { useState, useRef } from 'react';

const STATUS_CLASS = {
  critical: 'zone-bed-critical',
  warning: 'zone-bed-warning',
  normal: 'zone-bed-normal',
};

export default function ZoneBed3D({ config, color, status, primaryNodeId, dimmed, focused, onClick, children }) {
  const [hovered, setHovered] = useState(false);
  const timerRef = useRef(null);

  const handleEnter = () => {
    clearTimeout(timerRef.current);
    setHovered(true);
  };
  const handleLeave = () => {
    timerRef.current = setTimeout(() => setHovered(false), 100);
  };

  const childrenArr = Array.isArray(children) ? children : [children];
  const fault = status === 'critical' || status === 'warning';

  return (
    <div
      className={`zone-bed-3d ${STATUS_CLASS[status] || ''}${dimmed ? ' zone-dimmed' : ''}${focused ? ' zone-focused' : ''}`}
      data-zone-id={config.id}
      style={{
        left: config.x,
        top: config.y,
        width: config.w,
        height: config.d,
        borderColor: color,
      }}
      onMouseEnter={handleEnter}
      onMouseLeave={handleLeave}
      onClick={(e) => { e.stopPropagation(); onClick?.(); }}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClick?.(); } }}
      aria-label={`${config.name}，状态：${status === 'critical' ? '故障' : status === 'warning' ? '预警' : '正常'}`}
    >
      <div className="zone-bed-top" style={{ borderColor: color }}>
        <div className="zone-bed-name">
          {config.name}
          {fault && <span className={`zone-fault-mark ${status === 'critical' ? 'fault-critical' : 'fault-warning'}`}>
            {status === 'critical' ? '⚠' : '!'}
          </span>}
        </div>
        <div className="zone-bed-sensors">
          {childrenArr.map(child => {
            const isPrimary = child?.key === primaryNodeId;
            const visible = hovered || isPrimary;
            return (
              <div
                key={child?.key}
                className={`sensor-wrapper${visible ? '' : ' sensor-hidden'}`}
                style={{
                  left: child?.props?.config?.x,
                  top: child?.props?.config?.y,
                }}
              >
                {child}
              </div>
            );
          })}
        </div>
      </div>
      <div className="zone-bed-wall zone-bed-wall-right" style={{ borderColor: color, background: `${color}15` }} />
      <div className="zone-bed-wall zone-bed-wall-bottom" style={{ borderColor: color, background: `${color}15` }} />
    </div>
  );
}
