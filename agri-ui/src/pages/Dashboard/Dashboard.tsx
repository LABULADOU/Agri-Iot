import React, { useEffect } from 'react';
import { Row, Col, Card, Typography, Statistic } from 'antd';
import {
  ExperimentOutlined,
  WarningOutlined,
  CheckCircleOutlined,
} from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
import { useWeatherStore } from '../../stores/zoneStore';
import ZoneCard from '../../components/ZoneCard';
import WeatherPanel from '../../components/WeatherPanel';
import type { Zone, SensorNode, WeatherData } from '../../types';
import styles from './Dashboard.module.css';

const { Title } = Typography;

const MOCK_ZONES: Zone[] = [
  {
    id: '1',
    name: 'A区 - 番茄大棚',
    description: '主要种植番茄',
    location: '基地东北角',
    cropType: '番茄',
    comfortConfig: {
      airTemp: { min: 18, max: 28 },
      airHumidity: { min: 60, max: 80 },
      soilTemp: { min: 15, max: 25 },
      soilMoisture: { min: 40, max: 70 },
      ecValue: { min: 1.5, max: 3.5 },
    },
    nodeIds: ['n1', 'n2'],
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  },
  {
    id: '2',
    name: 'B区 - 黄瓜大棚',
    description: '主要种植黄瓜',
    location: '基地西北角',
    cropType: '黄瓜',
    comfortConfig: {
      airTemp: { min: 20, max: 30 },
      airHumidity: { min: 70, max: 90 },
      soilTemp: { min: 18, max: 28 },
      soilMoisture: { min: 50, max: 80 },
      ecValue: { min: 1.8, max: 4.0 },
    },
    nodeIds: ['n3'],
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  },
  {
    id: '3',
    name: 'C区 - 草莓温室',
    description: '草莓种植示范区',
    location: '基地中央',
    cropType: '草莓',
    comfortConfig: {
      airTemp: { min: 15, max: 25 },
      airHumidity: { min: 65, max: 85 },
      soilTemp: { min: 12, max: 22 },
      soilMoisture: { min: 45, max: 75 },
      ecValue: { min: 1.2, max: 3.0 },
    },
    nodeIds: ['n4', 'n5'],
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  },
];

const MOCK_WEATHER: WeatherData = {
  location: '北京',
  temp: 26,
  humidity: 65,
  text: '多云',
  windSpeed: 3.5,
  windDir: '东南风',
  updateTime: new Date().toLocaleTimeString('zh-CN'),
  forecast: [
    { date: '今天', tempMax: 30, tempMin: 22, textDay: '多云', textNight: '阴', humidity: 65 },
    { date: '明天', tempMax: 28, tempMin: 21, textDay: '晴', textNight: '晴', humidity: 60 },
    { date: '后天', tempMax: 29, tempMin: 23, textDay: '多云', textNight: '阴', humidity: 68 },
  ],
};

const MOCK_NODES: SensorNode[] = [
  { id: 'n1', name: '节点1', zoneId: '1', hasIrrigation: true, hasSideVent: true, hasRoofVent: true, ventRange: { min: 0, max: 100 }, sensors: [], status: 'online', lastSeen: new Date().toISOString() },
  { id: 'n2', name: '节点2', zoneId: '1', hasIrrigation: false, hasSideVent: true, hasRoofVent: false, ventRange: { min: 10, max: 90 }, sensors: [], status: 'online', lastSeen: new Date().toISOString() },
  { id: 'n3', name: '节点3', zoneId: '2', hasIrrigation: true, hasSideVent: false, hasRoofVent: true, ventRange: { min: 0, max: 100 }, sensors: [], status: 'offline', lastSeen: new Date().toISOString() },
  { id: 'n4', name: '节点4', zoneId: '3', hasIrrigation: true, hasSideVent: true, hasRoofVent: true, ventRange: { min: 5, max: 95 }, sensors: [], status: 'online', lastSeen: new Date().toISOString() },
  { id: 'n5', name: '节点5', zoneId: '3', hasIrrigation: false, hasSideVent: false, hasRoofVent: false, ventRange: { min: 0, max: 100 }, sensors: [], status: 'online', lastSeen: new Date().toISOString() },
];

const Dashboard: React.FC = () => {
  const navigate = useNavigate();
  const { current: weather, fetchWeather } = useWeatherStore();
  const nodes = MOCK_NODES;

  useEffect(() => {
    fetchWeather('101010100').catch(console.error);
    const timer = setInterval(() => fetchWeather('101010100').catch(console.error), 300000);
    return () => clearInterval(timer);
  }, [fetchWeather]);

  const displayZones = MOCK_ZONES;
  const displayWeather = weather || MOCK_WEATHER;

  const totalNodes = nodes.length;
  const onlineNodes = nodes.filter(n => n.status === 'online').length;
  const warningNodes = 0;

  const getZoneStats = (zoneId: string) => {
    const zoneNodes = nodes.filter(n => n.zoneId === zoneId);
    return {
      nodeCount: zoneNodes.length,
      onlineCount: zoneNodes.filter(n => n.status === 'online').length,
    };
  };

  return (
    <div className={styles.container}>
      <Title level={4}>数据总览</Title>

      <Row gutter={16} className={styles.statsRow}>
        <Col span={6}>
          <Card>
            <Statistic
              title="区域数量"
              value={displayZones.length}
              prefix={<ExperimentOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="在线节点"
              value={onlineNodes}
              suffix={`/ ${totalNodes}`}
              valueStyle={{ color: onlineNodes === totalNodes ? '#52c41a' : '#faad14' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="预警数量"
              value={warningNodes}
              prefix={<WarningOutlined />}
              valueStyle={{ color: warningNodes > 0 ? '#ff4d4f' : '#52c41a' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="设备正常率"
              value={Math.round((onlineNodes / totalNodes) * 100)}
              suffix="%"
              prefix={<CheckCircleOutlined />}
            />
          </Card>
        </Col>
      </Row>

      <Row gutter={16}>
        <Col span={6}>
          <WeatherPanel weather={displayWeather} />
        </Col>
        <Col span={18}>
          <Card title="区域概览">
            <Row gutter={16}>
              {displayZones.map(zone => {
                const stats = getZoneStats(zone.id);
                return (
                  <Col span={8} key={zone.id}>
                    <ZoneCard
                      zone={zone}
                      nodeCount={stats.nodeCount}
                      onlineCount={stats.onlineCount}
                      avgTemp={25 + Math.random() * 5}
                      avgHumidity={60 + Math.random() * 20}
                      comfort="optimal"
                      onClick={() => navigate(`/zones/${zone.id}`)}
                    />
                  </Col>
                );
              })}
            </Row>
          </Card>
        </Col>
      </Row>
    </div>
  );
};

export default Dashboard;