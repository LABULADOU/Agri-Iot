import React from 'react';
import { Typography, Badge, Tooltip } from 'antd';
import { WarningOutlined, CheckCircleOutlined } from '@ant-design/icons';
import type { Zone } from '../../../types';
import styles from './ZoneOverviewRow.module.css';

const { Text } = Typography;

const statusColors: Record<string, string> = {
  optimal: 'var(--status-optimal, #22C55E)',
  warning: 'var(--status-warning, #F59E0B)',
  danger: 'var(--status-danger, #EF4444)',
  offline: 'var(--status-offline, #9CA3AF)',
};

interface ZoneOverviewRowProps {
  zone: Zone;
  nodeName?: string;
  assessment?: { score: number; status: string };
  onlineCount: number;
  totalCount: number;
  latestReadings?: {
    airTemp?: number;
    humidity?: number;
    soilTemp?: number;
    soilMoisture?: number;
    ec?: number;
  };
  status?: string;
  anomalyCount?: number;
  anomalySeverity?: string;
  onClick?: () => void;
}

const ZoneOverviewRow: React.FC<ZoneOverviewRowProps> = ({
  zone,
  nodeName,
  assessment,
  onlineCount,
  totalCount,
  latestReadings = {},
  status = 'optimal',
  anomalyCount = 0,
  anomalySeverity,
  onClick,
}) => {
  const isOffline = totalCount > 0 && onlineCount === 0;
  const isAlert = status === 'danger' || status === 'warning';

  return (
    <div
      className={`${styles.row} ${isAlert ? styles.alert : ''} ${isOffline ? styles.offline : ''}`}
      onClick={onClick}
    >
      <span className={styles.dot} style={{ background: isOffline ? statusColors.offline : statusColors[status] }} />
      <span className={styles.zoneCol}>
        <Text strong>{zone.name}</Text>
      </span>
      <span className={styles.nodeCol}>
        <Text>{nodeName || zone.cropType || '--'}</Text>
      </span>
      <span className={styles.metric} data-label="气温">
        <Text>{latestReadings.airTemp?.toFixed(1) ?? '--'}℃</Text>
      </span>
      <span className={styles.metric} data-label="湿度">
        <Text>{latestReadings.humidity?.toFixed(0) ?? '--'}%</Text>
      </span>
      <span className={styles.metric} data-label="地温">
        <Text>{latestReadings.soilTemp?.toFixed(1) ?? '--'}℃</Text>
      </span>
      <span className={styles.metric} data-label="土壤">
        <Text>{latestReadings.soilMoisture?.toFixed(0) ?? '--'}%</Text>
      </span>
      <span className={styles.metric} data-label="EC">
        <Text>{latestReadings.ec?.toFixed(2) ?? '--'}mS/cm</Text>
      </span>
      <span className={styles.nodes}>
        <Badge status={isOffline ? 'error' : 'success'} />
        <Text type="secondary">{onlineCount}/{totalCount}</Text>
      </span>
      <span className={styles.sensorHealth}>
        {anomalyCount > 0 ? (
          <Tooltip title={`${anomalyCount} 个传感器异常 (${anomalySeverity || 'Warning'})`}>
            <span className={styles.anomalyBadge}>
              <WarningOutlined style={{ color: anomalySeverity === 'Critical' ? '#EF4444' : '#F59E0B' }} />
              <Text type="warning" style={{ fontSize: 12, marginLeft: 2 }}>{anomalyCount}</Text>
            </span>
          </Tooltip>
        ) : (
          <CheckCircleOutlined style={{ color: '#22C55E', fontSize: 14 }} />
        )}
      </span>
      <span className={styles.suggestion}>
        {assessment && assessment.score < 60 ? (
          <Text type="danger" className={styles.suggestText}>需关注</Text>
        ) : (
          <Text type="secondary" className={styles.suggestText}>正常</Text>
        )}
      </span>
    </div>
  );
};

export default ZoneOverviewRow;
