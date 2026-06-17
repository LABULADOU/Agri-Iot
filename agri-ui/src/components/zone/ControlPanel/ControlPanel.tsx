import React, { useState } from 'react';
import { Button, Slider, Typography, message } from 'antd';
import { deviceApi } from '../../../services/api';
import type { SensorNode } from '../../../types';
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

  const caps = node.capabilities ?? [];

  const sendCommand = async (deviceId: string, command: string, params?: Record<string, unknown>) => {
    setLoading(command);
    try {
      await deviceApi.sendCommand(deviceId, command, params);
      message.success('命令已发送');
      onStatusChange?.(deviceId, command);
    } catch {
      message.error('命令发送失败');
    } finally {
      setLoading(null);
    }
  };

  const hasControl = caps.includes('actuator');

  return (
    <div className={styles.panel}>
      <Text strong className={styles.title}>控制面板</Text>

      <div className={styles.block}>
        <div className={styles.btnGroup}>
          <Button type="primary" loading={loading === 'switch_on'} onClick={() => sendCommand(node.id, 'switch', { on: true })}>开启</Button>
          <Button danger loading={loading === 'switch_off'} onClick={() => sendCommand(node.id, 'switch', { on: false })}>关闭</Button>
        </div>
      </div>
    </div>
  );
};

export default ControlPanel;
