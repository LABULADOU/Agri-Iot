import React, { useEffect } from 'react';
import { Typography } from 'antd';
import { useNavigate } from 'react-router-dom';
import { useDashboardStore } from '../../stores/dashboardStore';
import HealthScoreBar from '../../components/HealthScoreBar';
import ZoneOverviewRow from '../../components/ZoneOverviewRow';
import EmergencyBanner from '../../components/EmergencyBanner';
import TodoList from '../../components/TodoList';
import AISummaryPanel from '../../components/AISummaryPanel';
import type { Zone } from '../../types';
import styles from './Dashboard.module.css';

const { Title } = Typography;

const MOCK_ZONES: (Zone & { status?: string; avgTemp?: number; avgHumidity?: number; avgSoilMoisture?: number })[] = [
  { id: '1', name: 'A区 - 番茄大棚', cropType: '番茄', location: '基地东北角', description: '', comfortConfig: { airTemp: {min:18,max:28}, airHumidity:{min:60,max:80}, soilTemp:{min:15,max:25}, soilMoisture:{min:40,max:70}, ecValue:{min:1.5,max:3.5} }, nodeIds: ['n1','n2'], createdAt: '', updatedAt: '', status: 'warning', avgTemp: 25.3, avgHumidity: 72, avgSoilMoisture: 38 },
  { id: '2', name: 'B区 - 黄瓜大棚', cropType: '黄瓜', location: '基地西北角', description: '', comfortConfig: { airTemp: {min:20,max:30}, airHumidity:{min:70,max:90}, soilTemp:{min:18,max:28}, soilMoisture:{min:50,max:80}, ecValue:{min:1.8,max:4.0} }, nodeIds: ['n3'], createdAt: '', updatedAt: '', status: 'optimal', avgTemp: 26.1, avgHumidity: 68, avgSoilMoisture: 55 },
  { id: '3', name: 'C区 - 草莓温室', cropType: '草莓', location: '基地中央', description: '', comfortConfig: { airTemp: {min:15,max:25}, airHumidity:{min:65,max:85}, soilTemp:{min:12,max:22}, soilMoisture:{min:45,max:75}, ecValue:{min:1.2,max:3.0} }, nodeIds: ['n4','n5'], createdAt: '', updatedAt: '', status: 'optimal', avgTemp: 22.8, avgHumidity: 75, avgSoilMoisture: 62 },
];

const Dashboard: React.FC = () => {
  const navigate = useNavigate();
  const { emergencies, todoItems, recommendations, healthScore, dismissEmergency, executeRecommendation } = useDashboardStore();

  useEffect(() => {
    useDashboardStore.getState().fetchAll();
    useDashboardStore.getState().fetchEmergencies();
    const timer = setInterval(() => {
      useDashboardStore.getState().fetchEmergencies();
    }, 30000);
    return () => clearInterval(timer);
  }, []);

  return (
    <div className={styles.container}>
      {emergencies.length > 0 && (
        <EmergencyBanner
          emergencies={emergencies}
          onDismiss={dismissEmergency}
          onViewDetail={(id) => navigate(`/zones/${id}`)}
        />
      )}

      <HealthScoreBar score={healthScore} trend={3} targetScore={80} />

      <div className={styles.tableHeader}>
        <Title level={5} style={{ margin: 0 }}>区域概览</Title>
      </div>

      <div className={styles.tableHeadRow}>
        <span />
        <span className={styles.th}>区域</span>
        <span className={styles.th}>气温</span>
        <span className={styles.th}>湿度</span>
        <span className={styles.th}>土壤</span>
        <span className={styles.th}>节点</span>
        <span className={styles.th}>状态</span>
      </div>

      <div className={styles.tableBody}>
        {MOCK_ZONES.map(zone => (
          <ZoneOverviewRow
            key={zone.id}
            zone={zone}
            status={zone.status}
            onlineCount={zone.nodeIds.length}
            totalCount={zone.nodeIds.length}
            latestReadings={{
              airTemp: zone.avgTemp,
              humidity: zone.avgHumidity,
              soilMoisture: zone.avgSoilMoisture,
            }}
            onClick={() => navigate(`/zones/${zone.id}`)}
          />
        ))}
      </div>

      <div className={styles.bottomGrid}>
        <div className={styles.bottomSection}>
          <Title level={5} style={{ margin: 0, marginBottom: 8 }}>待处理事项</Title>
          <TodoList items={todoItems} onExecute={executeRecommendation} />
        </div>
        <div className={styles.bottomSection}>
          <Title level={5} style={{ margin: 0, marginBottom: 8 }}>AI 摘要</Title>
          <AISummaryPanel recommendations={recommendations} />
        </div>
      </div>
    </div>
  );
};

export default Dashboard;
