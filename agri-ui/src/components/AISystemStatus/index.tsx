import React from 'react';
import { Typography, Badge } from 'antd';
import styles from './AISystemStatus.module.css';

const { Text } = Typography;

interface AISystemStatusProps {
  autoModeEnabled: boolean;
  nightModeActive: boolean;
  aiEnabled: boolean;
}

const statusItems: { key: string; label: string; getValue: (p: AISystemStatusProps) => boolean }[] = [
  { key: 'auto', label: '自动模式', getValue: p => p.autoModeEnabled },
  { key: 'night', label: '夜间模式', getValue: p => p.nightModeActive },
  { key: 'ai', label: 'AI 决策', getValue: p => p.aiEnabled },
];

const AISystemStatus: React.FC<AISystemStatusProps> = (props) => {
  return (
    <div className={styles.row}>
      {statusItems.map(item => {
        const active = item.getValue(props);
        return (
          <div key={item.key} className={styles.item}>
            <Badge status={active ? 'success' : 'default'} />
            <Text>{item.label}</Text>
            <Text type="secondary" className={styles.state}>{active ? '开启' : '关闭'}</Text>
          </div>
        );
      })}
    </div>
  );
};

export default AISystemStatus;
