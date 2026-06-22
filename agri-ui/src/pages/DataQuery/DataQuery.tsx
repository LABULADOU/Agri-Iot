import React, { useEffect, useState, useMemo } from 'react';
import { Row, Col, Select, Segmented, Space, Table, Typography, Statistic, DatePicker, Badge } from 'antd';
import dayjs from 'dayjs';
import { dataApi, nodeApi } from '../../services/api';
import LineChart from '../../components/Charts/LineChart';
import { metricSelectOptions, METRIC_CONFIG } from '../../config/metrics';
import { useRealtimeReadings } from '../../hooks/useRealtimeReadings';
import type { SensorNode, AggregatedReading, ViewMode } from '../../types';
import styles from './DataQuery.module.css';

const { Title, Text } = Typography;
const { RangePicker } = DatePicker;

const metricOptions = metricSelectOptions;

const viewModes: Record<ViewMode, { period: 'hour' | '10min'; defaultRangeHours: number; isRealtime: boolean }> = {
  realtime: { period: 'hour',  defaultRangeHours: 24,  isRealtime: true },
  ten_min:  { period: '10min', defaultRangeHours: 72,  isRealtime: false },
  daily:    { period: 'hour',  defaultRangeHours: 168, isRealtime: false },
};

const DataQuery: React.FC = () => {
  const [nodes, setNodes] = useState<SensorNode[]>([]);
  const [data, setData] = useState<AggregatedReading[]>([]);
  const [selectedNode, setSelectedNode] = useState<string>('');
  const [selectedMetrics, setSelectedMetrics] = useState<string[]>(['temperature', 'humidity']);
  const [viewMode, setViewMode] = useState<ViewMode>('realtime');
  const [dateRange, setDateRange] = useState<[dayjs.Dayjs, dayjs.Dayjs]>([
    dayjs().subtract(24, 'hour'),
    dayjs(),
  ]);

  const selectedNodeId = useMemo(() => {
    if (!selectedNode || selectedNode === 'all') return null;
    return nodes.find(n => n.id === selectedNode)?.node_id || null;
  }, [selectedNode, nodes]);

  const isRealtime = viewMode === 'realtime';
  const cfg = viewModes[viewMode];

  const {
    readings: realtimeReadings,
    filteredReadings: realtimeTableData,
    loading: realtimeLoading,
    lastUpdate,
    rawCount,
  } = useRealtimeReadings({
    enabled: isRealtime,
    deviceId: isRealtime && selectedNode && selectedNode !== 'all' ? selectedNode : null,
    nodeId: isRealtime && selectedNodeId ? selectedNodeId : null,
    metrics: selectedMetrics,
    dateRange,
  });

  useEffect(() => {
    fetchNodes();
  }, []);

  useEffect(() => {
    setDateRange([dayjs().subtract(cfg.defaultRangeHours, 'hour'), dayjs()]);
  }, [viewMode]);

  useEffect(() => {
    if (isRealtime) return;
    fetchData();
  }, [selectedNode, selectedMetrics, viewMode, dateRange]);

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

  const fetchData = async () => {
    try {
      const params: Record<string, unknown> = {
        period: cfg.period,
        start: dateRange[0].toISOString(),
        end: dayjs().toISOString(),
      };
      if (selectedNode && selectedNode !== 'all') params.node_id = selectedNode;
      if (selectedMetrics.length > 0) params.metric = selectedMetrics.join(',');
      const result = await dataApi.query(params as unknown as import('../../types').QueryParams);
      setData(result);
    } catch {
      setData([]);
    }
  };

  const chartData = useMemo(() => {
    if (isRealtime) return realtimeReadings;
    if (!selectedMetrics.length) return [];
    return data
      .filter(d => selectedMetrics.includes(d.metric))
      .sort((a, b) => dayjs(a.timestamp).valueOf() - dayjs(b.timestamp).valueOf());
  }, [isRealtime, realtimeReadings, data, selectedMetrics]);

  const statsData = useMemo(() => {
    const source = isRealtime ? realtimeTableData : data;
    return selectedMetrics.map(metric => {
      const rows = source.filter((r: any) => r.metric === metric);
      if (!rows.length) return null;
      const values = isRealtime
        ? rows.map((r: any) => r.value)
        : rows.flatMap((r: any) => [r.max, r.min, r.avg]);
      const allMax = Math.max(...values);
      const allMin = Math.min(...values);
      const avg = isRealtime
        ? values[values.length - 1]
        : values.reduce((a: number, b: number) => a + b, 0) / values.length;
      return { metric, max: allMax, min: allMin, avg };
    }).filter(Boolean);
  }, [isRealtime, realtimeTableData, data, selectedMetrics]);

  const [tablePage, setTablePage] = useState(1);

  useEffect(() => {
    if (!isRealtime || !lastUpdate) return;
    setTablePage(1);
  }, [lastUpdate, isRealtime]);

  const tableColumns = isRealtime
    ? [
        { title: '时间', dataIndex: 'timestamp', key: 'timestamp', width: 180, render: (t: string | number) => dayjs(t).format('YYYY-MM-DD HH:mm:ss') },
        { title: '指标', dataIndex: 'metric', key: 'metric', width: 100, render: (m: string) => metricOptions.find(o => o.value === m)?.label || m },
        { title: '数值', dataIndex: 'value', key: 'value', width: 120, render: (v: number) => v?.toFixed(2) },
        { title: '单位', dataIndex: 'unit', key: 'unit', width: 80 },
      ]
    : [
        { title: '时间', dataIndex: 'timestamp', key: 'timestamp', width: 180, render: (t: string | number) => dayjs(t).format('YYYY-MM-DD HH:mm') },
        { title: '指标', dataIndex: 'metric', key: 'metric', width: 100, render: (m: string) => metricOptions.find(o => o.value === m)?.label || m },
        { title: '最大值', dataIndex: 'max', key: 'max', width: 100, render: (v: number) => v?.toFixed(2) },
        { title: '最小值', dataIndex: 'min', key: 'min', width: 100, render: (v: number) => v?.toFixed(2) },
        { title: '平均值', dataIndex: 'avg', key: 'avg', width: 100, render: (v: number) => v?.toFixed(2) },
        { title: '样本数', dataIndex: 'count', key: 'count', width: 80 },
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
                options={[
                  { value: 'realtime', label: '实时' },
                  { value: 'ten_min',  label: '按小时' },
                  { value: 'daily',    label: '按天' },
                ]}
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
                disabled={isRealtime}
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
                suffix={METRIC_CONFIG[stat.metric]?.unit ?? ''}
                precision={2}
              />
              <Text type="secondary" className={styles.statRange}>
                最高 {stat.max.toFixed(1)} / 最低 {stat.min.toFixed(1)}
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
          data={chartData}
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
                text={`${rawCount} 条`}
                style={{ marginLeft: 12, fontWeight: 'normal' }}
              />
              <Text type="secondary" style={{ marginLeft: 8, fontSize: 12 }}>
                最后更新: {new Date(lastUpdate).toLocaleTimeString()}
              </Text>
            </>
          )}
        </Text>
        <Table
          columns={tableColumns}
          dataSource={(isRealtime ? realtimeTableData : chartData).slice().reverse() as any}
          rowKey={(_record, index) => `${index}`}
          loading={isRealtime ? realtimeLoading : false}
          size="small"
          pagination={{ pageSize: 20, showSizeChanger: true, current: tablePage, onChange: (p) => setTablePage(p) }}
          scroll={{ x: 700 }}
        />
      </div>
    </div>
  );
};

export default DataQuery;
