import React from 'react';
import { Progress, Card, Typography, Space, Tooltip } from 'antd';
import { InfoCircleOutlined } from '@ant-design/icons';
import type { ComfortConfig } from '../../types';
import styles from './ComfortIndicator.module.css';

const { Text } = Typography;

interface ComfortIndicatorProps {
  config: ComfortConfig;
  values: {
    airTemp?: number;
    airHumidity?: number;
    soilTemp?: number;
    soilMoisture?: number;
    ecValue?: number;
  };
  compact?: boolean;
}

const metricLabels: Record<string, string> = {
  airTemp: '空气温度',
  airHumidity: '空气湿度',
  soilTemp: '土壤温度',
  soilMoisture: '土壤湿度',
  ecValue: 'EC值',
};

const metricUnits: Record<string, string> = {
  airTemp: '℃',
  airHumidity: '%',
  soilTemp: '℃',
  soilMoisture: '%',
  ecValue: 'dS/m',
};

const ComfortIndicator: React.FC<ComfortIndicatorProps> = ({ config, values, compact = false }) => {
  const getComfortStatus = (value: number | undefined, min: number, max: number): 'success' | 'exception' => {
    if (value === undefined) return 'exception';
    if (value >= min && value <= max) return 'success';
    return 'exception';
  };

  const getPercentage = (value: number | undefined, min: number, max: number): number => {
    if (value === undefined) return 0;
    const range = max - min;
    const idealMin = min;
    const idealMax = max;
    if (value < idealMin) return Math.max(0, 50 - ((idealMin - value) / range) * 50);
    if (value > idealMax) return Math.min(100, 50 + ((value - idealMax) / range) * 50);
    return 50 + ((value - idealMin) / range) * 50;
  };

  const renderItem = (key: keyof typeof values, configItem: { min: number; max: number }) => {
    const value = values[key];
    const status = getComfortStatus(value, configItem.min, configItem.max);
    const percentage = getPercentage(value, configItem.min, configItem.max);
    const unit = metricUnits[key];

    if (compact) {
      return (
        <div key={key} className={styles.compactItem}>
          <Text type="secondary" className={styles.label}>{metricLabels[key]}</Text>
          <Progress
            percent={percentage}
            status={status}
            size="small"
            format={() => value !== undefined ? `${value.toFixed(1)}${unit}` : '--'}
          />
        </div>
      );
    }

    return (
      <div key={key} className={styles.item}>
        <div className={styles.itemHeader}>
          <Text>{metricLabels[key]}</Text>
          <Tooltip title={`舒适区间: ${configItem.min}${unit} - ${configItem.max}${unit}`}>
            <InfoCircleOutlined className={styles.infoIcon} />
          </Tooltip>
        </div>
        <Progress
          percent={percentage}
          status={status}
          format={() => value !== undefined ? `${value.toFixed(1)}${unit}` : '--'}
        />
        <Text type="secondary" className={styles.range}>
          舒适区间: {configItem.min} ~ {configItem.max} {unit}
        </Text>
      </div>
    );
  };

  return (
    <Card title="舒适度指示" className={styles.card} size="small">
      <Space direction="vertical" size="middle" style={{ width: '100%' }}>
        {renderItem('airTemp', config.airTemp)}
        {renderItem('airHumidity', config.airHumidity)}
        {renderItem('soilTemp', config.soilTemp)}
        {renderItem('soilMoisture', config.soilMoisture)}
        {renderItem('ecValue', config.ecValue)}
      </Space>
    </Card>
  );
};

export default ComfortIndicator;