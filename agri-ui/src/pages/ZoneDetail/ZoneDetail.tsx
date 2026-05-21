import React, { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Typography, Button, Space } from 'antd';
import { ArrowLeftOutlined } from '@ant-design/icons';
import { zoneApi, nodeApi, accTempApi } from '../../services/api';
import MetricRow from '../../components/MetricRow';
import ControlPanel from '../../components/ControlPanel';
import AIAnalysisReport from '../../components/AIAnalysisReport';
import OperationTimeline from '../../components/OperationTimeline';
import type { Zone, SensorNode, AccumulatedTemp } from '../../types';
import styles from './ZoneDetail.module.css';

const { Title, Text } = Typography;

interface FakeReading {
  label: string;
  key: string;
  value: number;
  unit: string;
  min: number;
  max: number;
}

const ZoneDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [zone, setZone] = useState<Zone | null>(null);
  const [nodes, setNodes] = useState<SensorNode[]>([]);
  const [accTemps, setAccTemps] = useState<AccumulatedTemp[]>([]);
  const [selectedNode, setSelectedNode] = useState<SensorNode | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (id) fetchData();
  }, [id]);

  const fetchData = async () => {
    if (!id) return;
    setLoading(true);
    try {
      const [zoneData, nodesData, tempsData] = await Promise.all([
        zoneApi.get(id),
        nodeApi.list(id),
        accTempApi.list(id).catch(() => [] as AccumulatedTemp[]),
      ]);
      setZone(zoneData);
      setNodes(nodesData);
      setAccTemps(tempsData);
      if (nodesData.length > 0) setSelectedNode(nodesData[0]);
    } catch (err) {
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  const readings: FakeReading[] = [
    { label: '空气温度', key: 'airTemp', value: 25.3, unit: '℃', min: 18, max: 28 },
    { label: '空气湿度', key: 'humidity', value: 72, unit: '%', min: 60, max: 80 },
    { label: '土壤温度', key: 'soilTemp', value: 21.5, unit: '℃', min: 15, max: 25 },
    { label: '土壤湿度', key: 'soilMoisture', value: 38, unit: '%', min: 40, max: 70 },
    { label: 'EC值', key: 'ecValue', value: 3.2, unit: 'dS/m', min: 1.5, max: 3.5 },
  ];

  const getStatus = (v: number, min: number, max: number): 'normal' | 'warning' | 'danger' => {
    const margin = (max - min) * 0.2;
    if (v < min - margin || v > max + margin) return 'danger';
    if (v < min || v > max) return 'warning';
    return 'normal';
  };

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <Space>
          <Button icon={<ArrowLeftOutlined />} onClick={() => navigate('/')}>返回</Button>
          <Title level={4} style={{ margin: 0 }}>{zone?.name || '区域详情'}</Title>
          <Text type="secondary">在线 {nodes.filter(n => n.status === 'online').length}/{nodes.length}</Text>
          {nodes.length > 0 && (
            <Text type="secondary">最后通讯 {nodes[0].lastSeen ? new Date(nodes[0].lastSeen).toLocaleTimeString('zh-CN') : '--'}</Text>
          )}
        </Space>
      </div>

      <div className={styles.mainGrid}>
        <div className={styles.metricsCol}>
          <div className={styles.metricsCard}>
            {readings.map(r => (
              <MetricRow
                key={r.key}
                label={r.label}
                value={r.value}
                unit={r.unit}
                status={getStatus(r.value, r.min, r.max)}
                range={{ min: r.min, max: r.max }}
                aiRecommendation={r.key === 'soilMoisture' ? '启动灌溉 20min' : r.key === 'ecValue' ? '接近上限，建议检测' : undefined}
                onExecuteRecommendation={() => console.log('execute', r.key)}
              />
            ))}
          </div>

          <AIAnalysisReport
            assessment={{
              score: 65,
              status: 'warning',
              summary: '当前土壤湿度 38%，低于阈值，建议启动灌溉 20 分钟。EC 值接近上限，建议检测水质。',
              details: ['土壤湿度低于下限 40%', 'EC 值 3.2 接近上限 3.5'],
            }}
            similarCases={[
              { id: 'c1', title: '05-15 灌溉记录', summary: '土壤湿度 35% → 启动灌溉 25min → 恢复至 55%', date: '2026-05-15' },
            ]}
          />

          <div className={styles.section}>
            <Text strong className={styles.sectionTitle}>历史操作记录</Text>
            <OperationTimeline
              records={[
                { id: 'op1', timestamp: '10:30', action: '启动灌溉 20min', result: 'success', aiGenerated: true },
                { id: 'op2', timestamp: '09:15', action: '打开侧窗 50%', result: 'success', aiGenerated: true },
                { id: 'op3', timestamp: '08:00', action: 'AI 评估', result: 'success', aiGenerated: true },
              ]}
            />
          </div>
        </div>

        <div className={styles.controlCol}>
          {selectedNode ? (
            <div className={styles.controlCard}>
              <ControlPanel node={selectedNode} />
            </div>
          ) : (
            <Text type="secondary">请选择一个节点</Text>
          )}
        </div>
      </div>
    </div>
  );
};

export default ZoneDetail;
