import React from 'react';
import { Typography, Button } from 'antd';
import styles from './MetricRow.module.css';

const { Text } = Typography;

interface MetricRowProps {
  label: string;
  value: number;
  unit: string;
  status: 'normal' | 'warning' | 'danger';
  range: { min: number; max: number };
  maxScale?: number;
  aiRecommendation?: string;
  onExecuteRecommendation?: () => void;
}

const statusColors: Record<string, string> = {
  normal: '#22C55E',
  warning: '#F59E0B',
  danger: '#EF4444',
};

const MetricRow: React.FC<MetricRowProps> = ({
  label,
  value,
  unit,
  status,
  range,
  maxScale,
  aiRecommendation,
  onExecuteRecommendation,
}) => {
  const scaleMax = maxScale ?? range.max * 2;
  const pct = Math.min(Math.max((value / scaleMax) * 100, 0), 100);
  const isAlert = status === 'danger' || status === 'warning';

  return (
    <div className={`${styles.row} ${isAlert ? styles.alert : ''}`}>
      <Text className={styles.label}>{label}</Text>
      <Text className={styles.value} style={{ color: statusColors[status] }}>
        {value.toFixed(1)}{unit}
      </Text>
      <div className={styles.track}>
        <div
          className={styles.fill}
          style={{ width: `${pct}%`, background: statusColors[status] }}
        />
      </div>
      <Text className={styles.statusTag} style={{ color: statusColors[status] }}>
        {status === 'normal' ? '正常' : status === 'warning' ? '注意' : '告警'}
      </Text>
      {aiRecommendation && (
        <div className={styles.aiBlock}>
          <Text type="secondary" className={styles.aiText}>AI: {aiRecommendation}</Text>
          {onExecuteRecommendation && (
            <Button size="small" type="primary" onClick={onExecuteRecommendation}>执行</Button>
          )}
        </div>
      )}
    </div>
  );
};

export default MetricRow;
