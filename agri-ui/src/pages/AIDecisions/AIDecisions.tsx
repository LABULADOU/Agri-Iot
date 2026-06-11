import React, { useState, useEffect, useCallback } from 'react';
import { Typography, Input, Card, Spin, Tag, Empty } from 'antd';
import { ClockCircleOutlined } from '@ant-design/icons';
import AISystemStatus from '../../components/AISystemStatus';
import EmergencyRules from '../../components/EmergencyRules';
import KnowledgeStats from '../../components/KnowledgeStats';
import { aiApi } from '../../services/api';
import type { EmergencyRuleResponse, KnowledgeSearchResult, ControlCaseRecord } from '../../types';
import styles from './AIDecisions.module.css';

const { Title, Text } = Typography;
const { Search } = Input;

const EMERGENCY_RULE_DEFS = [
  { id: 'e1', name: '大风保护', condition: '风速 > 40km/h', action: '关闭顶部通风' },
  { id: 'e2', name: '大雨保护', condition: '降雨 > 10mm/h', action: '关闭顶部通风' },
  { id: 'e3', name: '低温保护', condition: '气温 < 0℃', action: '关闭通风+暂停自动' },
];

const AIDecisions: React.FC = () => {
  const [emergencyStatus, setEmergencyStatus] = useState<{
    active: EmergencyRuleResponse[];
    nightMode: boolean;
    pausesAuto: boolean;
  } | null>(null);
  const [cases, setCases] = useState<ControlCaseRecord[]>([]);
  const [searchResults, setSearchResults] = useState<KnowledgeSearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      aiApi.emergencyStatus(),
      aiApi.knowledgeCases(10),
    ]).then(([status, caseList]) => {
      setEmergencyStatus({
        active: status.active_emergencies,
        nightMode: status.night_mode_active,
        pausesAuto: status.pauses_auto_mode,
      });
      setCases(caseList);
    }).finally(() => setLoading(false));
  }, []);

  const handleSearch = useCallback(async (value: string) => {
    if (!value.trim()) {
      setSearchResults([]);
      return;
    }
    setSearching(true);
    try {
      const results = await aiApi.knowledgeSearch(value);
      setSearchResults(results);
    } catch {
      setSearchResults([]);
    } finally {
      setSearching(false);
    }
  }, []);

  const rules = EMERGENCY_RULE_DEFS.map(def => ({
    ...def,
    active: emergencyStatus?.active.some(e => {
      const type = e.type.toLowerCase();
      return (
        (type.includes('wind') && def.id === 'e1') ||
        (type.includes('rain') && def.id === 'e2') ||
        (type.includes('snow') && def.id === 'e3') ||
        (type.includes('cold') && def.id === 'e3')
      );
    }) ?? false,
  }));

  if (loading) {
    return <Spin style={{ display: 'block', margin: '40px auto' }} />;
  }

  const caseCount = cases.length;
  const thisMonthNew = cases.filter(c => {
    const d = new Date(c.timestamp * 1000);
    const now = new Date();
    return d.getFullYear() === now.getFullYear() && d.getMonth() === now.getMonth();
  }).length;

  return (
    <div className={styles.container}>
      <Title level={4}>AI 决策中枢</Title>

      <div className={styles.statsRow}>
        <Card size="small" className={styles.statCard}>
          <Text type="secondary" className={styles.statLabel}>系统状态</Text>
          <AISystemStatus
            autoModeEnabled={!emergencyStatus?.pausesAuto}
            nightModeActive={emergencyStatus?.nightMode ?? false}
            aiEnabled
          />
        </Card>
        <Card size="small" className={styles.statCard}>
          <Text type="secondary" className={styles.statLabel}>紧急规则状态</Text>
          <EmergencyRules rules={rules} />
        </Card>
        <Card size="small" className={styles.statCard}>
          <Text type="secondary" className={styles.statLabel}>知识库统计</Text>
          <KnowledgeStats cropCount={12} pestCount={8} caseCount={caseCount} thisMonthNew={thisMonthNew} />
        </Card>
      </div>

      <Card size="small" className={styles.section}>
        <Title level={5}>知识库检索</Title>
        <Search
          placeholder="搜索作物、病虫害、调控案例..."
          allowClear
          enterButton="搜索"
          onSearch={handleSearch}
          loading={searching}
          className={styles.search}
        />
        {searchResults.length > 0 && (
          <div className={styles.results}>
            {searchResults.map((r) => (
              <div key={r.id} className={styles.resultItem}>
                <Tag color={r.type === 'crop_profile' ? 'green' : r.type === 'pest_knowledge' ? 'red' : 'blue'}>
                  {r.type === 'crop_profile' ? '作物' : r.type === 'pest_knowledge' ? '病虫害' : '气象'}
                </Tag>
                <Text>{r.name || r.condition_type}</Text>
              </div>
            ))}
          </div>
        )}
        {searchResults.length === 0 && !searching && (
          <Text type="secondary">输入关键词搜索知识库</Text>
        )}
      </Card>

      <Card size="small" className={styles.section}>
        <Title level={5}>调控案例记录</Title>
        {cases.length > 0 ? (
          <div className={styles.results}>
            {cases.map(c => (
              <div key={c.id} className={styles.resultItem}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <Text strong>{c.situation ? JSON.parse(c.situation).soil_temp ? `土壤温度 ${JSON.parse(c.situation).soil_temp}°C` : c.situation.slice(0, 50) : '-'}</Text>
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    <ClockCircleOutlined style={{ marginRight: 4 }} />
                    {new Date(c.timestamp * 1000).toLocaleDateString()}
                  </Text>
                </div>
                {c.outcome && (
                  <div style={{ marginTop: 4 }}>
                    <Text type="secondary">效果: </Text>
                    <Text>{c.outcome}</Text>
                    {c.effect_rating && (
                      <Tag color={c.effect_rating >= 4 ? 'green' : c.effect_rating >= 2 ? 'orange' : 'red'} style={{ marginLeft: 8 }}>
                        {c.effect_rating}/5
                      </Tag>
                    )}
                  </div>
                )}
              </div>
            ))}
          </div>
        ) : (
          <Empty description="暂无案例数据" image={Empty.PRESENTED_IMAGE_SIMPLE} />
        )}
      </Card>
    </div>
  );
};

export default AIDecisions;
