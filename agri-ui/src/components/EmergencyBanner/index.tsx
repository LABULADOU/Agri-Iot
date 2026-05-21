import React from 'react';
import { Button, Typography, Space } from 'antd';
import styles from './EmergencyBanner.module.css';

const { Text } = Typography;

interface EmergencyItem {
  id: string;
  type: string;
  message: string;
  severity: string;
}

interface EmergencyBannerProps {
  emergencies: EmergencyItem[];
  onDismiss?: (id: string) => void;
  onViewDetail?: (id: string) => void;
}

const EmergencyBanner: React.FC<EmergencyBannerProps> = ({
  emergencies,
  onDismiss,
  onViewDetail,
}) => {
  if (!emergencies.length) return null;

  return (
    <div className={styles.banner}>
      <div className={styles.inner}>
        {emergencies.map(e => (
          <div key={e.id} className={styles.item}>
            <span className={styles.icon}>⚠️</span>
            <Text className={styles.type}>{e.type}</Text>
            <Text className={styles.message}>{e.message}</Text>
            <Space size="small">
              <Button
                size="small"
                ghost
                className={styles.btn}
                onClick={() => onViewDetail?.(e.id)}
              >
                查看详情
              </Button>
              <Button
                size="small"
                ghost
                className={styles.btn}
                onClick={() => onDismiss?.(e.id)}
              >
                已确认
              </Button>
            </Space>
          </div>
        ))}
      </div>
    </div>
  );
};

export default EmergencyBanner;
