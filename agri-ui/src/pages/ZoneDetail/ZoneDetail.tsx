import React, { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Typography, Button, Space } from 'antd';
import { ArrowLeftOutlined } from '@ant-design/icons';
import { zoneApi, nodeApi } from '../../services/api';
import { useRealtimeStore } from '../../stores/realtimeStore';
import MetricRow from '../../components/MetricRow';
import ControlPanel from '../../components/ControlPanel';
import type { Zone, SensorNode, SensorReading } from '../../types';
import styles from './ZoneDetail.module.css';

const { Title, Text } = Typography;

interface DisplayReading {
  label: string;
  key: string;
  value: number | null;
  unit: string;
  min: number;
  max: number;
}

const METRIC_CONFIG: Record<string, { label: string; unit: string; min: number; max: number }> = {
  temperature: { label: '空气温度', unit: '℃', min: 18, max: 28 },
  humidity: { label: '空气湿度', unit: '%', min: 60, max: 80 },
  soil_temperature: { label: '土壤温度', unit: '℃', min: 15, max: 25 },
  soil_moisture: { label: '土壤湿度', unit: '%', min: 40, max: 70 },
  ec: { label: 'EC值', unit: 'mS/cm', min: 1.5, max: 3.5 },
  light: { label: '光照', unit: 'lux', min: 0, max: 200000 },
};

const ZoneDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [zone, setZone] = useState<Zone | null>(null);
  const [nodes, setNodes] = useState<SensorNode[]>([]);
  const [selectedNode, setSelectedNode] = useState<SensorNode | null>(null);
  const [readings, setReadings] = useState<DisplayReading[]>([]);
  const [loading, setLoading] = useState(true);
  const realtimeReadings = useRealtimeStore(s => s.readings);

  useEffect(() => {
    if (id) fetchData();
  }, [id]);

  useEffect(() => {
    if (!selectedNode) return;
    const nodeId = selectedNode.node_id;
    if (!nodeId) return;
    const nodeData = realtimeReadings.get(nodeId);
    if (!nodeData || nodeData.length === 0) return;
    const latest = new Map<string, number>();
    for (const r of nodeData) {
      latest.set(r.metric, r.value);
    }
    setReadings(prev => {
      if (prev.length === 0) return prev;
      const updated = prev.map(d => {
        const v = latest.get(d.key);
        return v !== undefined ? { ...d, value: v } : d;
      });
      const changed = updated.some((d, i) => d.value !== prev[i].value);
      return changed ? updated : prev;
    });
  }, [realtimeReadings, selectedNode]);

  const fetchData = async () => {
    if (!id) return;
    setLoading(true);
    try {
      const [zoneData, nodesData] = await Promise.all([
        zoneApi.get(id),
        nodeApi.list(id),
      ]);
      setZone(zoneData);
      setNodes(nodesData);
      if (nodesData.length > 0) {
        setSelectedNode(nodesData[0]);
        fetchReadings(nodesData[0].id);
      }
    } catch (err) {
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  const fetchReadings = async (deviceId: string) => {
    try {
      const data = await nodeApi.getReadings(deviceId, { limit: 50 });
      const latestByMetric = new Map<string, SensorReading>();
      for (const r of data) {
        const existing = latestByMetric.get(r.metric);
        if (!existing || r.id > existing.id) {
          latestByMetric.set(r.metric, r);
        }
      }
      const display: DisplayReading[] = [];
      for (const [metric, cfg] of Object.entries(METRIC_CONFIG)) {
        const reading = latestByMetric.get(metric);
        display.push({
          label: cfg.label,
          key: metric,
          value: reading?.value ?? null,
          unit: reading?.unit ?? cfg.unit,
          min: cfg.min,
          max: cfg.max,
        });
      }
      setReadings(display);
    } catch {
      setReadings([]);
    }
  };

  const getStatus = (v: number | null, min: number, max: number): 'normal' | 'warning' | 'danger' => {
    if (v === null) return 'danger';
    const margin = (max - min) * 0.2;
    if (v < min - margin || v > max + margin) return 'danger';
    if (v < min || v > max) return 'warning';
    return 'normal';
  };

  const onlineCount = nodes.filter(n => n.status === 'online').length;
  const latestTs = readings.length > 0 ? null : null;

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <Space>
          <Button icon={<ArrowLeftOutlined />} onClick={() => navigate('/')}>返回</Button>
          <Title level={4} style={{ margin: 0 }}>{zone?.name || '区域详情'}</Title>
          <Text type="secondary">在线 {onlineCount}/{nodes.length}</Text>
        </Space>
      </div>

      <div className={styles.mainGrid}>
        <div className={styles.metricsCol}>
          <div className={styles.metricsCard}>
            {readings.map(r => (
              <MetricRow
                key={r.key}
                label={r.label}
                value={r.value ?? 0}
                unit={r.unit}
                status={getStatus(r.value, r.min, r.max)}
                range={{ min: r.min, max: r.max }}
              />
            ))}
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
