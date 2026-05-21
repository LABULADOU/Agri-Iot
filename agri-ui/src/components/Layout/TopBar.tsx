import React from 'react';
import { Layout, Space, Typography, Badge } from 'antd';
import { useRealtimeStore } from '../../stores/realtimeStore';
import styles from './TopBar.module.css';

const { Text } = Typography;
const { Header: AntHeader } = Layout;

const weatherIcons: Record<string, string> = {
  '晴': '☀️', '多云': '⛅', '阴': '☁️',
  '雨': '🌧️', '雪': '❄️', '雾': '🌫️', '雷': '⛈️',
};

const mockWeather = { temp: 26, text: '多云', windSpeed: 3.5 };

const TopBar: React.FC = () => {
  const { connected, lastUpdate } = useRealtimeStore();
  const now = new Date();
  const timeStr = `${now.getHours().toString().padStart(2, '0')}:${now.getMinutes().toString().padStart(2, '0')}`;

  return (
    <AntHeader className={styles.topbar}>
      <div />
      <Space size="middle" className={styles.center}>
        <Text className={styles.weatherIcon}>
          {weatherIcons[mockWeather.text] || '🌤️'}
        </Text>
        <Text strong>{mockWeather.temp}℃</Text>
        <Text type="secondary">{mockWeather.text}</Text>
        <Text type="secondary" className={styles.divider}>|</Text>
        <Text type="secondary">{mockWeather.windSpeed}m/s</Text>
      </Space>
      <Space size="middle">
        <Badge count={0} showZero size="small" className={styles.alertBadge} />
        <Text type="secondary" className={styles.time}>{timeStr}</Text>
        <Badge status={connected ? 'success' : 'error'} />
        <Text type="secondary" className={styles.connText}>
          {connected ? '在线' : '离线'}
        </Text>
        {lastUpdate && (
          <Text type="secondary" className={styles.updateTime}>
            {new Date(lastUpdate).toLocaleTimeString('zh-CN')}
          </Text>
        )}
      </Space>
    </AntHeader>
  );
};

export default TopBar;
