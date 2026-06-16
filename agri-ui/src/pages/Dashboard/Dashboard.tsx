import React, { useEffect } from 'react';
import { Typography } from 'antd';
import { useNavigate } from 'react-router-dom';
import { useDashboardStore } from '../../stores/dashboardStore';
import HealthScoreBar from '../../components/HealthScoreBar';
import ZoneOverviewRow from '../../components/ZoneOverviewRow';
import EmergencyBanner from '../../components/EmergencyBanner';
import TodoList from '../../components/TodoList';
import AISummaryPanel from '../../components/AISummaryPanel';
import styles from './Dashboard.module.css';

const { Title } = Typography;

const Dashboard: React.FC = () => {
  const navigate = useNavigate();
  const { zones, assessments, emergencies, todoItems, recommendations, healthScore, healthTrend, nodeReadings, dismissEmergency, executeRecommendation } = useDashboardStore();

  useEffect(() => {
    useDashboardStore.getState().fetchAll();
    useDashboardStore.getState().fetchEmergencies();
    const timer = setInterval(() => {
      useDashboardStore.getState().fetchEmergencies();
    }, 30000);
    return () => {
      clearInterval(timer);
      useDashboardStore.getState().stopRealtimeUpdates();
    };
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

      <HealthScoreBar score={healthScore} trend={healthTrend} targetScore={80} />

      <div className={styles.tableHeader}>
        <Title level={5} style={{ margin: 0 }}>区域概览</Title>
      </div>

      <div className={styles.tableHeadRow}>
        <span />
        <span className={styles.thLeft}>区域</span>
        <span className={styles.thLeft}>节点名称</span>
        <span className={styles.th}>气温</span>
        <span className={styles.th}>湿度</span>
        <span className={styles.th}>地温</span>
        <span className={styles.th}>土壤</span>
        <span className={styles.th}>EC</span>
        <span className={styles.th}>状态</span>
      </div>

      <div className={styles.tableBody}>
        {nodeReadings.map((nr, i) => {
          const assessment = assessments[nr.zoneId];
          const zone = zones.find(z => z.id === nr.zoneId);
          const isOnline = nr.status === 'online';
          return (
            <ZoneOverviewRow
              key={`${nr.nodeId}-${i}`}
              zone={zone || { id: nr.zoneId, name: nr.zoneName }}
              nodeName={nr.nodeName}
              assessment={assessment ? { score: assessment.score, status: assessment.status } : undefined}
              onlineCount={isOnline ? 1 : 0}
              totalCount={1}
              latestReadings={nr.readings}
              onClick={() => navigate(`/zones/${nr.zoneId}`)}
            />
          );
        })}
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
