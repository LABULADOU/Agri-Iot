import React, { useState } from 'react';
import { Typography, Input, Card } from 'antd';
import AISystemStatus from '../../components/AISystemStatus';
import EmergencyRules from '../../components/EmergencyRules';
import KnowledgeStats from '../../components/KnowledgeStats';
import styles from './AIDecisions.module.css';

const { Title, Text } = Typography;
const { Search } = Input;

const mockEmergencyRules = [
  { id: 'e1', name: '大风保护', condition: '风速 > 40km/h', active: false, action: '关闭顶部通风' },
  { id: 'e2', name: '大雨保护', condition: '降雨 > 10mm/h', active: true, action: '关闭顶部通风' },
  { id: 'e3', name: '低温保护', condition: '气温 < 0℃', active: false, action: '关闭通风+暂停自动' },
];

const AIDecisions: React.FC = () => {
  const [searchResults, setSearchResults] = useState<string[]>([]);

  const handleSearch = (value: string) => {
    if (!value.trim()) {
      setSearchResults([]);
      return;
    }
    setSearchResults([
      `番茄：昼夜温差管理（2026-05-15）`,
      `黄瓜：湿度控制与病害预防（2026-05-10）`,
      `草莓：EC 值监测与调整（2026-05-08）`,
    ]);
  };

  return (
    <div className={styles.container}>
      <Title level={4}>AI 决策中枢</Title>

      <div className={styles.statsRow}>
        <Card size="small" className={styles.statCard}>
          <Text type="secondary" className={styles.statLabel}>系统状态</Text>
          <AISystemStatus autoModeEnabled nightModeActive={false} aiEnabled />
        </Card>
        <Card size="small" className={styles.statCard}>
          <Text type="secondary" className={styles.statLabel}>紧急规则状态</Text>
          <EmergencyRules rules={mockEmergencyRules} />
        </Card>
        <Card size="small" className={styles.statCard}>
          <Text type="secondary" className={styles.statLabel}>知识库统计</Text>
          <KnowledgeStats cropCount={12} pestCount={8} caseCount={156} thisMonthNew={12} />
        </Card>
      </div>

      <Card size="small" className={styles.section}>
        <Title level={5}>知识库检索</Title>
        <Search
          placeholder="搜索作物、病虫害、调控案例..."
          allowClear
          enterButton="搜索"
          onSearch={handleSearch}
          className={styles.search}
        />
        {searchResults.length > 0 && (
          <div className={styles.results}>
            {searchResults.map((r, i) => (
              <div key={i} className={styles.resultItem}>
                <Text>{r}</Text>
              </div>
            ))}
          </div>
        )}
      </Card>

      <Card size="small" className={styles.section}>
        <Title level={5}>调控案例记录</Title>
        <Text type="secondary">暂无案例数据</Text>
      </Card>
    </div>
  );
};

export default AIDecisions;
