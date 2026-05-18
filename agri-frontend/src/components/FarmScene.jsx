import { useState, useRef, useCallback, useEffect } from 'react';
import { farmLayout } from '../config/farmLayout';
import ZoneBed3D from './ZoneBed3D';
import SensorNode3D from './SensorNode3D';

function matchDeviceNode(devices, sensorCfg) {
  if (!devices) return undefined;
  return devices.find(d => {
    const dn = d?.node_id;
    const cn = sensorCfg.nodeId;
    return dn === cn || d?.id === cn || (dn && cn && (dn.endsWith(cn) || cn.endsWith(dn)));
  });
}

export default function FarmScene({ zoneDataMap, onZoneClick, selectedZoneId }) {
  const { zones } = farmLayout;
  const [rotateY, setRotateY] = useState(0);
  const [scale, setScale] = useState(1);
  const dragging = useRef(false);
  const lastX = useRef(0);
  const wasDrag = useRef(false);

  const handlePointerDown = useCallback((e) => {
    if (e.target.closest('.zone-bed-3d')) return;
    dragging.current = true;
    wasDrag.current = false;
    lastX.current = e.clientX;
    e.currentTarget.setPointerCapture(e.pointerId);
  }, []);

  const handlePointerMove = useCallback((e) => {
    if (!dragging.current) return;
    const dx = e.clientX - lastX.current;
    if (Math.abs(dx) > 3) wasDrag.current = true;
    lastX.current = e.clientX;
    setRotateY(prev => Math.max(-60, Math.min(60, prev + dx * 0.3)));
  }, []);

  const handlePointerUp = useCallback(() => {
    dragging.current = false;
  }, []);

  const sceneRef = useRef(null);

  useEffect(() => {
    const el = sceneRef.current;
    if (!el) return;
    const handler = (e) => {
      e.preventDefault();
      setScale(prev => {
        const factor = e.deltaY > 0 ? 0.92 : 1.08;
        return Math.max(0.4, Math.min(2.5, prev * factor));
      });
    };
    el.addEventListener('wheel', handler, { passive: false });
    return () => el.removeEventListener('wheel', handler);
  }, []);

  const resetView = useCallback(() => {
    setRotateY(0);
    setScale(1);
  }, []);

  const handleZoneClick = useCallback((zoneId) => {
    if (wasDrag.current) return;
    wasDrag.current = false;
    onZoneClick?.(zoneId === selectedZoneId ? null : zoneId);
  }, [onZoneClick, selectedZoneId]);

  return (
    <div
      ref={sceneRef}
      className="scene-container"
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerCancel={handlePointerUp}
      onDoubleClick={resetView}
    >
      {(scale !== 1 || rotateY !== 0) && (
        <div className="scene-controls">
          <button className="btn btn-sm scene-reset-btn" onClick={resetView}>
            重置视角
          </button>
          <span className="text-xs text-dim">双击重置</span>
        </div>
      )}

      <div
        className="scene-3d"
        style={{
          transform: `rotateY(${rotateY}deg) scale(${scale})`,
        }}
      >
        <div className="scene-ground" />

        {zones.map(zcfg => {
          const areaData = zoneDataMap?.[zcfg.id];
          const primaryNodeId = zcfg.sensors.find(s => s.primary)?.nodeId || zcfg.sensors[0]?.nodeId;
          const isFocused = selectedZoneId === zcfg.id;
          const isDimmed = selectedZoneId !== null && !isFocused;

          return (
            <ZoneBed3D
              key={zcfg.id}
              config={zcfg}
              active={!!areaData}
              color={zcfg.color}
              status={areaData?.status || 'normal'}
              primaryNodeId={primaryNodeId}
              dimmed={isDimmed}
              focused={isFocused}
              onClick={() => handleZoneClick(zcfg.id)}
            >
              {zcfg.sensors.map(sensorCfg => {
                const deviceData = matchDeviceNode(areaData?.devices, sensorCfg);
                return (
                  <SensorNode3D
                    key={sensorCfg.nodeId}
                    config={sensorCfg}
                    readings={deviceData?.readings || deviceData?.latest_readings || {}}
                    status={deviceData?.status}
                  />
                );
              })}
            </ZoneBed3D>
          );
        })}
      </div>
    </div>
  );
}
