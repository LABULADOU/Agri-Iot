import React from 'react';
import { Card, Typography, Badge, Space } from 'antd';
import { EnvironmentOutlined } from '@ant-design/icons';
import type { Zone } from '../../types';
import styles from './ZoneCard.module.css';

const { Text, Title } = Typography;

interface ZoneCardProps {
  zone: Zone;
  nodeCount: number;
  onlineCount: number;
  avgTemp?: number;
  avgHumidity?: number;
  comfort?: 'optimal' | 'warning' | 'danger';
  onClick: () => void;
}

const comfortLabels = {
  optimal: '舒适',
  warning: '预警',
  danger: '危险',
};

const comfortColors = {
  optimal: '#52c41a',
  warning: '#faad14',
  danger: '#ff4d4f',
};

const ZoneCard: React.FC<ZoneCardProps> = ({
  zone,
  nodeCount,
  onlineCount,
  avgTemp,
  avgHumidity,
  comfort = 'optimal',
  onClick,
}) => {
  return (
    <Card
      className={styles.card}
      hoverable
      onClick={onClick}
    >
      <div className={styles.header}>
        <Badge color={comfortColors[comfort]} />
        <Title level={5} className={styles.title}>{zone.name}</Title>
      </div>

      <Space direction="vertical" size={4} className={styles.info}>
        <Text type="secondary" className={styles.location}>
          <EnvironmentOutlined /> {zone.location}
        </Text>
        <Text type="secondary" className={styles.crop}>
          作物: {zone.cropType}
        </Text>
      </Space>

      <div className={styles.stats}>
        <div className={styles.stat}>
          <Text type="secondary">节点</Text>
          <Text strong>{onlineCount}/{nodeCount}</Text>
        </div>
        {avgTemp !== undefined && (
          <div className={styles.stat}>
            <Text type="secondary">温度</Text>
            <Text>{avgTemp.toFixed(1)}℃</Text>
          </div>
        )}
        {avgHumidity !== undefined && (
          <div className={styles.stat}>
            <Text type="secondary">湿度</Text>
            <Text>{avgHumidity.toFixed(0)}%</Text>
          </div>
        )}
      </div>

      <div className={styles.comfort}>
        <Text type="secondary" className={styles.comfortLabel}>
          状态: {comfortLabels[comfort]}
        </Text>
      </div>
    </Card>
  );
};

export default ZoneCard;