import React, { useState } from 'react';
import { Button, Slider, Typography, message } from 'antd';
import { controlApi } from '../../services/api';
import type { SensorNode } from '../../types';
import styles from './ControlPanel.module.css';

const { Text } = Typography;

interface ControlPanelProps {
  node: SensorNode;
  onStatusChange?: (deviceId: string, status: string) => void;
}

const ControlPanel: React.FC<ControlPanelProps> = ({ node, onStatusChange }) => {
  const [ventValues, setVentValues] = useState<Record<string, number>>({
    side: 50,
    roof: 50,
  });
  const [loading, setLoading] = useState<string | null>(null);

  const sendCommand = async (deviceId: string, command: string, payload?: Record<string, unknown>) => {
    setLoading(command);
    try {
      await controlApi.sendCommand({ deviceId, command: command as 'irrigation' | 'side_vent' | 'roof_vent', action: 'on' });
      message.success('命令已发送');
      onStatusChange?.(deviceId, command);
    } catch {
      message.error('命令发送失败');
    } finally {
      setLoading(null);
    }
  };

  const hasControl = node.hasIrrigation || node.hasSideVent || node.hasRoofVent;

  if (!hasControl) {
    return (
      <div className={styles.panel}>
        <Text type="secondary" className={styles.empty}>此节点无可控制设备</Text>
      </div>
    );
  }

  return (
    <div className={styles.panel}>
      <Text strong className={styles.title}>控制面板</Text>

      {node.hasSideVent && (
        <div className={styles.block}>
          <div className={styles.blockHeader}>
            <Text>侧窗通风</Text>
            <Text className={styles.ventValue}>{ventValues.side}%</Text>
          </div>
          <Slider
            value={ventValues.side}
            min={node.ventRange.min}
            max={node.ventRange.max}
            onChange={(v) => setVentValues(prev => ({ ...prev, side: v as number }))}
            className={styles.slider}
          />
          <div className={styles.btnGroup}>
            <Button size="small" type="primary" ghost loading={loading === 'side_vent_on'} onClick={() => sendCommand(node.id, 'side_vent_on')}>打开</Button>
            <Button size="small" danger ghost loading={loading === 'side_vent_off'} onClick={() => sendCommand(node.id, 'side_vent_off')}>关闭</Button>
            <Button size="small" type="text" loading={loading === 'side_vent_calibrate'} onClick={() => sendCommand(node.id, 'side_vent_calibrate')}>校准</Button>
          </div>
        </div>
      )}

      {node.hasRoofVent && (
        <div className={styles.block}>
          <div className={styles.blockHeader}>
            <Text>顶部通风</Text>
            <Text className={styles.ventValue}>{ventValues.roof}%</Text>
          </div>
          <Slider
            value={ventValues.roof}
            min={node.ventRange.min}
            max={node.ventRange.max}
            onChange={(v) => setVentValues(prev => ({ ...prev, roof: v as number }))}
            className={styles.slider}
          />
          <div className={styles.btnGroup}>
            <Button size="small" type="primary" ghost loading={loading === 'roof_vent_on'} onClick={() => sendCommand(node.id, 'roof_vent_on')}>打开</Button>
            <Button size="small" danger ghost loading={loading === 'roof_vent_off'} onClick={() => sendCommand(node.id, 'roof_vent_off')}>关闭</Button>
            <Button size="small" type="text" loading={loading === 'roof_vent_calibrate'} onClick={() => sendCommand(node.id, 'roof_vent_calibrate')}>校准</Button>
          </div>
        </div>
      )}

      {node.hasIrrigation && (
        <div className={styles.block}>
          <div className={styles.blockHeader}>
            <Text>灌溉</Text>
            <Text className={styles.ventValue}>idle</Text>
          </div>
          <div className={styles.btnGroup}>
            <Button size="small" type="primary" loading={loading === 'irrigation_start'} onClick={() => sendCommand(node.id, 'irrigation_start')}>启动</Button>
            <Button size="small" danger loading={loading === 'irrigation_stop'} onClick={() => sendCommand(node.id, 'irrigation_stop')}>停止</Button>
          </div>
        </div>
      )}
    </div>
  );
};

export default ControlPanel;
