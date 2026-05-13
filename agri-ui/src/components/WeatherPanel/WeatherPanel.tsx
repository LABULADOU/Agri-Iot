import React from 'react';
import { Card, Typography, Row, Col } from 'antd';
import type { WeatherData } from '../../types';
import styles from './WeatherPanel.module.css';

const { Text } = Typography;

interface WeatherPanelProps {
  weather: WeatherData | null;
  loading?: boolean;
}

const getWeatherIcon = (text: string): string => {
  if (text.includes('晴')) return '☀️';
  if (text.includes('阴') || text.includes('多云')) return '⛅';
  if (text.includes('雨')) return '🌧️';
  if (text.includes('雪')) return '❄️';
  if (text.includes('雾') || text.includes('霾')) return '🌫️';
  if (text.includes('雷')) return '⛈️';
  return '🌤️';
};

const WeatherPanel: React.FC<WeatherPanelProps> = ({ weather }) => {
  if (!weather) {
    return (
      <Card className={styles.card}>
        <Text type="secondary">加载天气数据...</Text>
      </Card>
    );
  }

  return (
    <Card className={styles.card} title="天气预报">
      <div className={styles.current}>
        <div className={styles.currentMain}>
          <span className={styles.icon}>{getWeatherIcon(weather.text)}</span>
          <div>
            <div className={styles.temp}>{weather.temp}℃</div>
            <Text type="secondary">{weather.text}</Text>
          </div>
        </div>
        <div className={styles.currentInfo}>
          <div className={styles.infoItem}>
            <Text type="secondary">湿度</Text>
            <Text>{weather.humidity}%</Text>
          </div>
          <div className={styles.infoItem}>
            <Text type="secondary">风向</Text>
            <Text>{weather.windDir}</Text>
          </div>
          <div className={styles.infoItem}>
            <Text type="secondary">风速</Text>
            <Text>{weather.windSpeed} m/s</Text>
          </div>
        </div>
      </div>

      {weather.forecast?.length > 0 && (
        <div className={styles.forecast}>
          <Text type="secondary" className={styles.forecastTitle}>预报</Text>
          <Row gutter={16}>
            {weather.forecast.map((day, index) => (
              <Col span={8} key={index}>
                <div className={styles.forecastDay}>
                  <Text type="secondary">{index === 0 ? '今天' : day.date}</Text>
                  <span className={styles.forecastIcon}>{getWeatherIcon(day.textDay)}</span>
                  <div className={styles.forecastTemp}>
                    <Text>{day.tempMax}℃</Text>
                    <Text type="secondary"> / {day.tempMin}℃</Text>
                  </div>
                </div>
              </Col>
            ))}
          </Row>
        </div>
      )}

      <Text type="secondary" className={styles.updateTime}>
        更新: {weather.updateTime}
      </Text>
    </Card>
  );
};

export default WeatherPanel;