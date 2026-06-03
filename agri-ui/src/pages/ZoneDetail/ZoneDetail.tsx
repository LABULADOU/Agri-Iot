import React, { useEffect, useState, useCallback, useMemo } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Typography, Button, Space, Spin, Tag, Empty } from 'antd';
import { ArrowLeftOutlined, ReloadOutlined } from '@ant-design/icons';
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
  maxScale: number;
}

const METRIC_CONFIG: Record<string, { label: string; unit: string; min: number; max: number; maxScale: number }> = {
  temperature: { label: '空气温度', unit: '℃', min: 18, max: 28, maxScale: 50 },
  humidity: { label: '空气湿度', unit: '%', min: 60, max: 80, maxScale: 100 },
  soil_temperature: { label: '土壤温度', unit: '℃', min: 15, max: 25, maxScale: 50 },
  soil_moisture: { label: '土壤湿度', unit: '%', min: 40, max: 70, maxScale: 100 },
  ec: { label: 'EC值', unit: 'mS/cm', min: 1.5, max: 3.5, maxScale: 5 },
  light: { label: '光照', unit: 'lux', min: 0, max: 200000, maxScale: 200000 },
};

const METRIC_KEYS = Object.keys(METRIC_CONFIG);

function getStatus(v: number | null, min: number, max: number): 'normal' | 'warning' | 'danger' {
  if (v === null) return 'danger';
  const margin = (max - min) * 0.2;
  if (v < min - margin || v > max + margin) return 'danger';
  if (v < min || v > max) return 'warning';
  return 'normal';
}

const ZoneDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [zone, setZone] = useState<Zone | null>(null);
  const [nodes, setNodes] = useState<SensorNode[]>([]);
  const [selectedNode, setSelectedNode] = useState<SensorNode | null>(null);
  const [readings, setReadings] = useState<DisplayReading[]>([]);
  const [loading, setLoading] = useState(true);
  const [readingsLoading, setReadingsLoading] = useState(false);
  const realtimeReadings = useRealtimeStore(s => s.readings);

  const readingsMap = useMemo(() => {
    if (!selectedNode) return new Map<string, number>();
    const nodeId = selectedNode.node_id;
    if (!nodeId) return new Map<string, number>();
    const nodeData = realtimeReadings.get(nodeId);
    if (!nodeData) return new Map<string, number>();
    const map = new Map<string, number>();
    for (const r of nodeData) {
      map.set(r.metric, r.value);
    }
    return map;
  }, [realtimeReadings, selectedNode]);

  const fetchData = useCallback(async () => {
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
        setSelectedNode(prev => {
          const stillExists = prev && nodesData.some(n => n.id === prev.id);
          return stillExists ? prev : nodesData[0];
        });
      } else {
        setSelectedNode(null);
        setReadings([]);
      }
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }, [id]);

  const fetchReadings = useCallback(async (deviceId: string) => {
    setReadingsLoading(true);
    try {
      const data = await nodeApi.getReadings(deviceId, { limit: 50 });
      const latestByMetric = new Map<string, SensorReading>();
      for (const r of data) {
        const existing = latestByMetric.get(r.metric);
        if (!existing || r.id > existing.id) {
          latestByMetric.set(r.metric, r);
        }
      }
      const display: DisplayReading[] = METRIC_KEYS.map(key => {
        const cfg = METRIC_CONFIG[key];
        const reading = latestByMetric.get(key);
        return {
          label: cfg.label,
          key,
          value: reading?.value ?? null,
          unit: reading?.unit ?? cfg.unit,
          min: cfg.min,
          max: cfg.max,
          maxScale: cfg.maxScale,
        };
      });
      setReadings(display);
    } catch {
      setReadings([]);
    } finally {
      setReadingsLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  useEffect(() => {
    if (selectedNode) {
      fetchReadings(selectedNode.id);
    }
  }, [selectedNode, fetchReadings]);

  const mergedReadings = useMemo(() => {
    if (readings.length === 0) return [];
    return readings.map(r => {
      const v = readingsMap.get(r.key);
      return v !== undefined ? { ...r, value: v } : r;
    });
  }, [readings, readingsMap]);

  const onlineCount = nodes.filter(n => n.status === 'online').length;
  const hasData = mergedReadings.some(r => r.value !== null);

  if (loading) {
    return <div style={{ display: 'flex', justifyContent: 'center', padding: 80 }}><Spin size="large" /></div>;
  }

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <Space wrap>
          <Button icon={<ArrowLeftOutlined />} onClick={() => navigate('/')}>返回</Button>
          <Title level={4} style={{ margin: 0 }}>{zone?.name || '区域详情'}</Title>
          <Tag color={onlineCount > 0 ? 'green' : 'default'}>
            在线 {onlineCount}/{nodes.length}
          </Tag>
          <Button size="small" icon={<ReloadOutlined />} onClick={fetchData}>刷新</Button>
        </Space>
      </div>

      {nodes.length > 0 && (
        <Space wrap size={[4, 4]} className={styles.nodeTabs}>
          {nodes.map(n => (
            <Tag
              key={n.id}
              color={selectedNode?.id === n.id ? 'blue' : 'default'}
              className={styles.nodeTag}
              onClick={() => setSelectedNode(n)}
              style={{ cursor: 'pointer' }}
            >
              {n.name || n.node_id}
            </Tag>
          ))}
        </Space>
      )}

      <div className={styles.mainGrid}>
        <div className={styles.metricsCol}>
          <div className={styles.metricsCard}>
            {readingsLoading ? (
              <div style={{ textAlign: 'center', padding: 24 }}><Spin /></div>
            ) : !hasData ? (
              <Empty description="暂无传感器数据" image={Empty.PRESENTED_IMAGE_SIMPLE} />
            ) : (
              mergedReadings.map(r => (
                <MetricRow
                  key={r.key}
                  label={r.label}
                  value={r.value ?? 0}
                  unit={r.unit}
                  status={getStatus(r.value, r.min, r.max)}
                  range={{ min: r.min, max: r.max }}
                  maxScale={r.maxScale}
                />
              ))
            )}
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
