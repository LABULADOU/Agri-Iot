import React from 'react';
import { Typography } from 'antd';
import styles from './OperationTimeline.module.css';

const { Text } = Typography;

interface OperationRecord {
  id: string;
  timestamp: string;
  action: string;
  result: 'success' | 'failed' | 'pending';
  aiGenerated?: boolean;
}

interface OperationTimelineProps {
  records: OperationRecord[];
}

const resultIcons: Record<string, string> = {
  success: '✓',
  failed: '✗',
  pending: '◷',
};

const resultColors: Record<string, string> = {
  success: '#22C55E',
  failed: '#EF4444',
  pending: '#F59E0B',
};

const OperationTimeline: React.FC<OperationTimelineProps> = ({ records }) => {
  if (!records.length) {
    return (
      <div className={styles.empty}>
        <Text type="secondary">暂无操作记录</Text>
      </div>
    );
  }

  return (
    <div className={styles.timeline}>
      {records.map(r => (
        <div key={r.id} className={styles.item}>
          <div className={styles.node} style={{ borderColor: resultColors[r.result] }}>
            <span style={{ color: resultColors[r.result] }}>{resultIcons[r.result]}</span>
          </div>
          <div className={styles.content}>
            <Text className={styles.action}>{r.action}</Text>
            <Text type="secondary" className={styles.time}>{r.timestamp}</Text>
            {r.aiGenerated && (
              <Text type="secondary" className={styles.aiBadge}>AI</Text>
            )}
          </div>
        </div>
      ))}
    </div>
  );
};

export default OperationTimeline;
