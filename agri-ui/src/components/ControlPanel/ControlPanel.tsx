import React from 'react';
import { Card, Switch, Slider, Space, Typography, message } from 'antd';
import {
  ThunderboltOutlined,
} from '@ant-design/icons';
import type { SensorNode } from '../../types';
import { controlApi } from '../../services/api';
import styles from './ControlPanel.module.css';

const { Text } = Typography;

interface ControlPanelProps {
  node: SensorNode;
  onStatusChange?: (deviceId: string, status: string) => void;
}

const ControlPanel: React.FC<ControlPanelProps> = ({ node, onStatusChange }) => {
  const [irrigationOn, setIrrigationOn] = React.useState(false);
  const [loading, setLoading] = React.useState<string | null>(null);

  const sendCommand = async (command: string, action: string | number) => {
    setLoading(command);
    try {
      await controlApi.sendCommand({
        deviceId: node.id,
        command: command as 'irrigation' | 'side_vent' | 'roof_vent',
        action: action as 'on' | 'off' | number,
      });
      message.success('命令已发送');
      onStatusChange?.(node.id, command);
    } catch {
      message.error('命令发送失败');
    } finally {
      setLoading(null);
    }
  };

  const hasControl = node.hasIrrigation || node.hasSideVent || node.hasRoofVent;

  return (
    <Card title="控制面板" className={styles.card}>
      <Space direction="vertical" size="large" style={{ width: '100%' }}>
        {node.hasIrrigation && (
          <div className={styles.controlItem}>
            <div className={styles.controlHeader}>
              <ThunderboltOutlined className={styles.icon} />
              <Text strong>灌溉控制</Text>
            </div>
            <div className={styles.controlBody}>
              <Switch
                checked={irrigationOn}
                onChange={(checked: boolean) => {
                  setIrrigationOn(checked);
                  sendCommand('irrigation', checked ? 'on' : 'off');
                }}
                loading={loading === 'irrigation'}
                checkedChildren="开"
                unCheckedChildren="关"
              />
              <Text type="secondary" className={styles.status}>
                {irrigationOn ? '已开启' : '已关闭'}
              </Text>
            </div>
          </div>
        )}

        {node.hasSideVent && (
          <div className={styles.controlItem}>
            <div className={styles.controlHeader}>
              <Text strong>侧通风帘</Text>
              <Text type="secondary" className={styles.range}>
                (量程: {node.ventRange.min}% - {node.ventRange.max}%)
              </Text>
            </div>
            <Slider
              min={node.ventRange.min}
              max={node.ventRange.max}
              defaultValue={50}
              onAfterChange={(val: number) => sendCommand('side_vent', val)}
              disabled={loading === 'side_vent'}
              marks={{
                [node.ventRange.min]: `${node.ventRange.min}%`,
                [Math.round((node.ventRange.min + node.ventRange.max) / 2)]: `${Math.round((node.ventRange.min + node.ventRange.max) / 2)}%`,
                [node.ventRange.max]: `${node.ventRange.max}%`,
              }}
            />
          </div>
        )}

        {node.hasRoofVent && (
          <div className={styles.controlItem}>
            <div className={styles.controlHeader}>
              <Text strong>顶部通风</Text>
              <Text type="secondary" className={styles.range}>
                (量程: {node.ventRange.min}% - {node.ventRange.max}%)
              </Text>
            </div>
            <Slider
              min={node.ventRange.min}
              max={node.ventRange.max}
              defaultValue={50}
              onAfterChange={(val: number) => sendCommand('roof_vent', val)}
              disabled={loading === 'roof_vent'}
              marks={{
                [node.ventRange.min]: `${node.ventRange.min}%`,
                [Math.round((node.ventRange.min + node.ventRange.max) / 2)]: `${Math.round((node.ventRange.min + node.ventRange.max) / 2)}%`,
                [node.ventRange.max]: `${node.ventRange.max}%`,
              }}
            />
          </div>
        )}

        {!hasControl && (
          <Text type="secondary">此节点无可控制设备</Text>
        )}
      </Space>
    </Card>
  );
};

export default ControlPanel;