import React, { useEffect, useState, useMemo, useRef } from 'react';
import { Row, Col, Select, Segmented, Space, Table, Typography, Statistic, DatePicker, Badge } from 'antd';
import dayjs from 'dayjs';
import { dataApi, nodeApi } from '../../services/api';
import LineChart from '../../components/Charts/LineChart';
import { wsService } from '../../services/ws';
import type { SensorNode, SensorReading, AggregatedReading, ViewMode } from '../../types';
import styles from './DataQuery.module.css';

const { Title, Text } = Typography;
const { RangePicker } = DatePicker;

const metricOptions = [
  { value: 'temperature', label: '空气温度' },
  { value: 'humidity', label: '空气湿度' },
  { value: 'soil_temperature', label: '土壤温度' },
  { value: 'soil_moisture', label: '土壤湿度' },
  { value: 'ec', label: 'EC值' },
];

interface ViewConfig {
  period: 'hour' | '10min';
  defaultRangeHours: number;
  label: string;
  isRealtime: boolean;
}
const viewModes: Record<ViewMode, ViewConfig> = {
  realtime: { period: 'hour',  defaultRangeHours: 24,  label: '实时', isRealtime: true },
  ten_min:  { period: '10min', defaultRangeHours: 72,  label: '按小时', isRealtime: false },
  daily:    { period: 'hour',  defaultRangeHours: 168, label: '按天', isRealtime: false },
};

const DataQuery: React.FC = () => {
  const [nodes, setNodes] = useState<SensorNode[]>([]);
  const [data, setData] = useState<AggregatedReading[]>([]);
  const [realtimeReadings, setRealtimeReadings] = useState<SensorReading[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedNode, setSelectedNode] = useState<string>('');
  const [selectedMetrics, setSelectedMetrics] = useState<string[]>(['temperature', 'humidity']);
  const [viewMode, setViewMode] = useState<ViewMode>('realtime');
  const [lastPollTime, setLastPollTime] = useState(Date.now());
  const [dateRange, setDateRange] = useState<[dayjs.Dayjs, dayjs.Dayjs]>([
    dayjs().subtract(24, 'hour'),
    dayjs(),
  ]);

  const selectedNodeId = useMemo(() => {
    if (!selectedNode || selectedNode === 'all') return null;
    return nodes.find(n => n.id === selectedNode)?.node_id || null;
  }, [selectedNode, nodes]);

  const seenKeys = useRef<Set<string>>(new Set());

  useEffect(() => {
    fetchNodes();
  }, []);

  const cfg = viewModes[viewMode];

  useEffect(() => {
    const dur = cfg.defaultRangeHours;
    setDateRange([dayjs().subtract(dur, 'hour'), dayjs()]);
  }, [viewMode]);

  useEffect(() => {
    if (viewMode === 'realtime') return;
    fetchData();
  }, [selectedNode, selectedMetrics, viewMode, dateRange]);

  useEffect(() => {
    if (viewMode !== 'realtime') {
      setRealtimeReadings([]);
      seenKeys.current.clear();
      return;
    }

    setRealtimeReadings([]);
    seenKeys.current.clear();
    fetchRealtimeInitial();

    console.log('[DataQuery] WS sub created, selectedNodeId:', selectedNodeId);
    const unsubscribe = wsService.subscribe('telemetry', selectedNodeId ? [selectedNodeId] : [], (msg) => {
      const msgNodeId = (msg.node_id as string) || '';
      console.log('[DataQuery] WS msg received:', msg.node_id, 'readings count:', (msg.readings as Array<unknown> || []).length);
      const readings = (msg.readings as Array<Record<string, string>> || [])
        .filter(r => r.metric && r.value != null)
        .map(r => ({
          id: Date.now() + Math.floor(Math.random() * 10000),
          device_id: msgNodeId,
          metric: r.metric as string,
          value: Number(r.value) || 0,
          unit: (r.unit as string) || '',
          timestamp: r.timestamp as string || new Date().toISOString(),
        }));

      if (!readings.length) return;

      setRealtimeReadings(prev => {
        const next = [...prev, ...readings.filter(r => {
          const key = `${r.metric}:${r.id}`;
          if (seenKeys.current.has(key)) return false;
          seenKeys.current.add(key);
          return true;
        })];
        const MAX_BUF = 30000;
        if (next.length > MAX_BUF) {
          const removed = next.slice(0, next.length - MAX_BUF);
          removed.forEach(r => seenKeys.current.delete(`${r.metric}:${r.id}`));
          return next.slice(-MAX_BUF);
        }
        return next;
      });
    });

    const pollInterval = setInterval(() => fetchRealtimeInitial(false), 10000);

    return () => {
      unsubscribe();
      clearInterval(pollInterval);
    };
  }, [viewMode, selectedNode, selectedNodeId]);

  const fetchNodes = async () => {
    try {
      const result = await nodeApi.list();
      setNodes(result);
      if (result.length > 0) {
        setSelectedNode(result[0].id);
      }
    } catch {
      setNodes([]);
    }
  };

  const fetchRealtimeInitial = async (showLoading = true) => {
    if (!selectedNode || selectedNode === 'all') return;
    if (showLoading) setLoading(true);
    try {
      const raw = await nodeApi.getReadings(selectedNode, {
        limit: 5000,
      });
      const mapped = raw.map(r => ({
        ...r,
        device_id: selectedNodeId || r.device_id,
        timestamp: typeof r.timestamp === 'number'
          ? (r.timestamp as number) * 1000
          : r.timestamp,
      }));
      setRealtimeReadings(mapped.reverse());
      mapped.forEach(r => seenKeys.current.add(`${r.metric}:${r.id}`));
      setLastPollTime(Date.now());
    } catch (e) {
      console.error('fetchRealtimeInitial error:', e);
    } finally {
      if (showLoading) setLoading(false);
    }
  };

  const fetchData = async () => {
    setLoading(true);
    try {
      const params: Record<string, unknown> = {
        period: cfg.period,
        start: dateRange[0].toISOString(),
        end: dateRange[1].toISOString(),
      };
      if (selectedNode && selectedNode !== 'all') params.node_id = selectedNode;
      if (selectedMetrics.length > 0) params.metric = selectedMetrics.join(',');
      const result = await dataApi.query(params as unknown as import('../../types').QueryParams);
      setData(result);
    } catch {
      setData([]);
    } finally {
      setLoading(false);
    }
  };

  const isRealtime = viewMode === 'realtime';

  const filteredRealtime = useMemo(() => {
    if (!isRealtime) return [];
    const start = dateRange[0].valueOf();
    const end = dateRange[1].valueOf();
    return realtimeReadings
      .filter(r => {
        const ts = dayjs(r.timestamp).valueOf();
        if (selectedNodeId && r.device_id !== selectedNodeId) return false;
        return selectedMetrics.includes(r.metric) && ts >= start && ts <= end;
      })
      .sort((a, b) => dayjs(a.timestamp).valueOf() - dayjs(b.timestamp).valueOf());
  }, [realtimeReadings, selectedMetrics, selectedNodeId, dateRange]);

  const filteredData = useMemo(() => {
    if (isRealtime) {
      return filteredRealtime.map(r => ({
        timestamp: r.timestamp,
        metric: r.metric,
        max: r.value,
        min: r.value,
        avg: r.value,
        count: 1,
      }));
    }
    if (!selectedMetrics.length) return [];
    return data
      .filter(d => selectedMetrics.includes(d.metric))
      .sort((a, b) => dayjs(a.timestamp).valueOf() - dayjs(b.timestamp).valueOf());
  }, [isRealtime ? realtimeReadings : data, selectedMetrics, filteredRealtime]);

  const statsData = useMemo(() => {
    if (isRealtime) {
      const start = dateRange[0].valueOf();
      const end = dateRange[1].valueOf();
      return selectedMetrics.map(metric => {
        const vals = realtimeReadings.filter(r => {
          const ts = dayjs(r.timestamp).valueOf();
          if (selectedNodeId && r.device_id !== selectedNodeId) return false;
          return r.metric === metric && ts >= start && ts <= end;
        });
        if (!vals.length) return null;
        const latest = vals[vals.length - 1];
        const allMax = Math.max(...vals.map(r => r.value));
        const allMin = Math.min(...vals.map(r => r.value));
        return { metric, max: allMax, min: allMin, avg: latest.value, label: '当前' };
      }).filter(Boolean);
    }
    return selectedMetrics.map(metric => {
      const metricData = data.filter(d => d.metric === metric);
      if (metricData.length === 0) return null;
      const values = metricData.flatMap(d => [d.max, d.min, d.avg]);
      const allMax = Math.max(...values);
      const allMin = Math.min(...values);
      const allAvg = values.reduce((a, b) => a + b, 0) / values.length;
      return { metric, max: allMax, min: allMin, avg: allAvg };
    }).filter(Boolean);
  }, [isRealtime ? realtimeReadings : data, selectedMetrics, selectedNodeId, dateRange]);

  const tableColumns = isRealtime
    ? [
        { title: '时间', dataIndex: 'timestamp', key: 'timestamp', width: 180, render: (t: string) => dayjs(t).format('YYYY-MM-DD HH:mm:ss') },
        { title: '指标', dataIndex: 'metric', key: 'metric', width: 100, render: (m: string) => metricOptions.find(o => o.value === m)?.label || m },
        { title: '数值', dataIndex: 'value', key: 'value', width: 120, render: (v: number) => v.toFixed(2) },
        { title: '单位', dataIndex: 'unit', key: 'unit', width: 80 },
      ]
    : [
        { title: '时间', dataIndex: 'timestamp', key: 'timestamp', width: 180, render: (t: string) => dayjs(t).format('YYYY-MM-DD HH:mm') },
        { title: '指标', dataIndex: 'metric', key: 'metric', width: 100, render: (m: string) => metricOptions.find(o => o.value === m)?.label || m },
        { title: '最大值', dataIndex: 'max', key: 'max', width: 100, render: (v: number) => v?.toFixed(2) },
        { title: '最小值', dataIndex: 'min', key: 'min', width: 100, render: (v: number) => v?.toFixed(2) },
        { title: '平均值', dataIndex: 'avg', key: 'avg', width: 100, render: (v: number) => v?.toFixed(2) },
        { title: '样本数', dataIndex: 'count', key: 'count', width: 80 },
      ];

  const segOptions: { value: ViewMode; label: string }[] = [
    { value: 'realtime', label: `实时` },
    { value: 'ten_min', label: `按小时` },
    { value: 'daily',   label: `按天` },
  ];

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <Title level={4}>数据查询</Title>
      </div>

      <div className={styles.filterCard}>
        <Row gutter={[8, 8]} className={styles.filterRow} align="middle">
          <Col xs={12} sm={12} lg={6}>
            <Space direction="vertical" size={2} style={{ width: '100%' }}>
              <Text type="secondary" className={styles.filterLabel}>采集节点</Text>
              <Select value={selectedNode} onChange={setSelectedNode} style={{ width: '100%' }}>
                <Select.Option value="all">全部节点</Select.Option>
                {nodes.map(n => <Select.Option key={n.id} value={n.id}>{n.name}</Select.Option>)}
              </Select>
            </Space>
          </Col>
          <Col xs={12} sm={12} lg={6}>
            <Space direction="vertical" size={2} style={{ width: '100%' }}>
              <Text type="secondary" className={styles.filterLabel}>监测指标</Text>
              <Select mode="multiple" value={selectedMetrics} onChange={setSelectedMetrics} style={{ width: '100%' }} maxTagCount={1}>
                {metricOptions.map(o => <Select.Option key={o.value} value={o.value}>{o.label}</Select.Option>)}
              </Select>
            </Space>
          </Col>
          <Col xs={12} sm={12} lg={6}>
            <Space direction="vertical" size={2} style={{ width: '100%' }}>
              <Text type="secondary" className={styles.filterLabel}>展示粒度</Text>
              <Segmented
                value={viewMode}
                onChange={(v) => setViewMode(v as ViewMode)}
                options={segOptions}
                size="small"
                block
              />
            </Space>
          </Col>
          <Col xs={12} sm={12} lg={6}>
            <Space direction="vertical" size={2} style={{ width: '100%' }}>
              <Text type="secondary" className={styles.filterLabel}>时间范围</Text>
              <RangePicker
                value={dateRange}
                onChange={(dates) => dates && setDateRange([dates[0]!, dates[1]!])}
                style={{ width: '100%' }}
                showTime={viewMode !== 'daily'}
                disabled={viewMode === 'realtime'}
                size="small"
              />
            </Space>
          </Col>
        </Row>
      </div>

      <Row gutter={[12, 12]} className={styles.statsRow}>
        {statsData.map(stat => stat && (
          <Col xs={12} sm={8} lg={4} key={stat.metric}>
            <div>
              <Statistic
                title={
                  <Space size={4}>
                    {metricOptions.find(o => o.value === stat.metric)?.label}
                    {isRealtime && <Badge status="success" />}
                  </Space>
                }
                value={stat.avg}
                suffix={(metricOptions.find(o => o.value === stat.metric)?.label.includes('温度') ? '℃' : '%')}
                precision={2}
              />
              <Text type="secondary" className={styles.statRange}>
                {isRealtime
                  ? `最高 ${stat.max.toFixed(1)} / 最低 ${stat.min.toFixed(1)}`
                  : `最高 ${stat.max.toFixed(1)} / 最低 ${stat.min.toFixed(1)}`
                }
              </Text>
            </div>
          </Col>
        ))}
      </Row>

      <div className={styles.chartCard}>
        <Text strong style={{ display: 'block', marginBottom: 12 }}>
          {isRealtime ? '实时数据趋势' : '数据趋势'}
        </Text>
        <LineChart
          key={`${viewMode}-${selectedMetrics.join(',')}`}
          data={filteredData}
          height={typeof window !== 'undefined' && window.innerWidth < 768 ? 220 : 400}
          showLegend={selectedMetrics.length > 1}
        />
      </div>

      <div className={styles.tableCard}>
        <Text strong style={{ display: 'block', marginBottom: 12 }}>
          {isRealtime ? '实时数据明细' : '数据明细'}
          {isRealtime && (
            <>
              <Badge
                status="processing"
                text={`${realtimeReadings.length} 条`}
                style={{ marginLeft: 12, fontWeight: 'normal' }}
              />
              <Text type="secondary" style={{ marginLeft: 8, fontSize: 12 }}>
                最后更新: {new Date(lastPollTime).toLocaleTimeString()}
              </Text>
            </>
          )}
        </Text>
        <Table
          columns={tableColumns}
          dataSource={((isRealtime ? filteredRealtime : filteredData).slice().reverse()) as any}
          rowKey={(_record, index) => `${index}`}
          loading={loading}
          size="small"
          pagination={{ pageSize: 20, showSizeChanger: true }}
          scroll={{ x: 700 }}
        />
      </div>
    </div>
  );
};

export default DataQuery;
