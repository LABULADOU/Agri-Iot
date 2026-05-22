import React, { useEffect, useState, useMemo } from 'react';
import { Row, Col, Select, DatePicker, Space, Table, Typography, Segmented, Statistic } from 'antd';
import dayjs from 'dayjs';
import { dataApi, nodeApi } from '../../services/api';
import LineChart from '../../components/Charts/LineChart';
import type { SensorNode, AggregatedReading, TimePeriod } from '../../types';
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

const DataQuery: React.FC = () => {
  const [nodes, setNodes] = useState<SensorNode[]>([]);
  const [data, setData] = useState<AggregatedReading[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedNode, setSelectedNode] = useState<string>('');
  const [selectedMetrics, setSelectedMetrics] = useState<string[]>(['temperature', 'humidity']);
  const [period, setPeriod] = useState<TimePeriod>('day');
  const [dateRange, setDateRange] = useState<[dayjs.Dayjs, dayjs.Dayjs]>([
    dayjs().subtract(1, 'day'),
    dayjs(),
  ]);

  useEffect(() => {
    fetchNodes();
  }, []);

  useEffect(() => {
    fetchData();
  }, [selectedNode, selectedMetrics, period, dateRange]);

  const fetchNodes = async () => {
    try {
      const result = await nodeApi.list();
      setNodes(result);
      if (result.length > 0) {
        setSelectedNode(result[0].id);
      }
    } catch (err) {
      console.error(err);
      setNodes([]);
    }
  };

  const fetchData = async () => {
    setLoading(true);
    try {
      const params: Record<string, unknown> = {
        period,
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

  const filteredData = useMemo(() => {
    if (!selectedMetrics.length) return [];
    return data.filter(d => selectedMetrics.includes(d.metric));
  }, [data, selectedMetrics]);

  const statsData = useMemo(() => {
    return selectedMetrics.map(metric => {
      const metricData = data.filter(d => d.metric === metric);
      if (metricData.length === 0) return null;
      const values = metricData.flatMap(d => [d.max, d.min, d.avg]);
      const allMax = Math.max(...values);
      const allMin = Math.min(...values);
      const allAvg = values.reduce((a, b) => a + b, 0) / values.length;
      const unit = metricOptions.find(o => o.value === metric)?.label.includes('温度') ? '℃' :
                   metricOptions.find(o => o.value === metric)?.label.includes('湿度') ? '%' : '';
      return { metric, max: allMax, min: allMin, avg: allAvg, unit };
    }).filter(Boolean);
  }, [data, selectedMetrics]);

  const tableColumns = [
    { title: '时间', dataIndex: 'timestamp', key: 'timestamp', width: 180, render: (t: string) => dayjs(t).format('YYYY-MM-DD HH:mm') },
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
        <Row gutter={16} align="middle">
          <Col span={6}>
            <Space direction="vertical" size={4}>
              <Text type="secondary">采集节点</Text>
              <Select value={selectedNode} onChange={setSelectedNode} style={{ width: '100%' }}>
                <Select.Option value="all">全部节点</Select.Option>
                {nodes.map(n => <Select.Option key={n.id} value={n.id}>{n.name}</Select.Option>)}
              </Select>
            </Space>
          </Col>
          <Col span={6}>
            <Space direction="vertical" size={4}>
              <Text type="secondary">监测指标</Text>
              <Select mode="multiple" value={selectedMetrics} onChange={setSelectedMetrics} style={{ width: '100%' }} maxTagCount={2}>
                {metricOptions.map(o => <Select.Option key={o.value} value={o.value}>{o.label}</Select.Option>)}
              </Select>
            </Space>
          </Col>
          <Col span={8}>
            <Space direction="vertical" size={4}>
              <Text type="secondary">时间范围</Text>
              <RangePicker value={dateRange} onChange={(dates) => dates && setDateRange([dates[0]!, dates[1]!])} style={{ width: '100%' }} />
            </Space>
          </Col>
          <Col span={4}>
            <Space direction="vertical" size={4}>
              <Text type="secondary">时间粒度</Text>
              <Segmented value={period} onChange={(v) => setPeriod(v as TimePeriod)} options={[
                { label: '小时', value: 'hour' },
                { label: '天', value: 'day' },
                { label: '周', value: 'week' },
                { label: '月', value: 'month' },
              ]} />
            </Space>
          </Col>
        </Row>
      </div>

      <Row gutter={16} className={styles.statsRow}>
        {statsData.map(stat => stat && (
          <Col span={4} key={stat.metric}>
            <div>
              <Statistic
                title={metricOptions.find(o => o.value === stat.metric)?.label}
                value={stat.avg}
                suffix={stat.unit}
                precision={2}
              />
              <Text type="secondary" className={styles.statRange}>
                最高 {stat.max.toFixed(1)}{stat.unit} / 最低 {stat.min.toFixed(1)}{stat.unit}
              </Text>
            </div>
          </Col>
        ))}
      </Row>

      <div className={styles.chartCard}>
        <Text strong style={{ display: 'block', marginBottom: 12 }}>数据趋势</Text>
        <LineChart key={selectedMetrics.join(',')} data={filteredData} height={400} showLegend={selectedMetrics.length > 1} />
      </div>

      <div className={styles.tableCard}>
        <Text strong style={{ display: 'block', marginBottom: 12 }}>数据明细</Text>
        <Table
          columns={tableColumns}
          dataSource={filteredData}
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