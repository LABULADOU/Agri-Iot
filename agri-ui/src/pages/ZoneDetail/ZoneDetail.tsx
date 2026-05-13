import React, { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Row, Col, Card, Typography, Space, Button, Descriptions, Tag, Table, Badge, Statistic, Progress } from 'antd';
import { ArrowLeftOutlined, ThunderboltOutlined, AimOutlined, FieldNumberOutlined } from '@ant-design/icons';
import { zoneApi, nodeApi, accTempApi } from '../../services/api';
import ComfortIndicator from '../../components/ComfortIndicator';
import ControlPanel from '../../components/ControlPanel';
import type { Zone, SensorNode, AccumulatedTemp } from '../../types';
import styles from './ZoneDetail.module.css';

const { Title, Text } = Typography;

const ZoneDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [zone, setZone] = useState<Zone | null>(null);
  const [nodes, setNodes] = useState<SensorNode[]>([]);
  const [accTemps, setAccTemps] = useState<AccumulatedTemp[]>([]);
  const [selectedNode, setSelectedNode] = useState<SensorNode | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (id) {
      fetchData();
    }
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
      if (nodesData.length > 0) {
        setSelectedNode(nodesData[0]);
      }
    } catch (err) {
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  const currentTemp = accTemps.length > 0 ? accTemps[accTemps.length - 1] : null;

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <Space>
          <Button icon={<ArrowLeftOutlined />} onClick={() => navigate('/zones')}>返回</Button>
          <Title level={4} style={{ margin: 0 }}>{zone?.name || '区域详情'}</Title>
        </Space>
      </div>

      <Row gutter={16} className={styles.infoRow}>
        <Col span={24}>
          <Card size="small">
            <Descriptions size="small" column={4}>
              <Descriptions.Item label="位置">{zone?.location || '-'}</Descriptions.Item>
              <Descriptions.Item label="作物">{zone?.cropType || '-'}</Descriptions.Item>
              <Descriptions.Item label="描述">{zone?.description || '-'}</Descriptions.Item>
              <Descriptions.Item label="节点数">
                <Badge status={nodes.length > 0 ? 'success' : 'default'} text={`${nodes.filter(n => n.status === 'online').length}/${nodes.length}`} />
              </Descriptions.Item>
            </Descriptions>
          </Card>
        </Col>
      </Row>

      <Row gutter={16}>
        <Col span={8}>
          <Card title="数据采集节点" size="small">
            <Table
              size="small"
              dataSource={nodes}
              rowKey="id"
              loading={loading}
              pagination={false}
              onRow={(record) => ({
                onClick: () => setSelectedNode(record),
                style: { cursor: 'pointer', background: selectedNode?.id === record.id ? '#e6f7ff' : undefined },
              })}
              columns={[
                { title: '名称', dataIndex: 'name' },
                { title: '状态', dataIndex: 'status', render: (s: string) => <Badge status={s === 'online' ? 'success' : 'error'} text={s === 'online' ? '在线' : '离线'} /> },
                {
                  title: '控制',
                  render: (_: unknown, record: SensorNode) => (
                    <Space size={4}>
                      {record.hasIrrigation && <Tag icon={<ThunderboltOutlined />} color="blue">灌</Tag>}
                      {record.hasSideVent && <Tag icon={<FieldNumberOutlined />} color="green">侧</Tag>}
                      {record.hasRoofVent && <Tag icon={<AimOutlined />} color="orange">顶</Tag>}
                    </Space>
                  ),
                },
              ]}
            />
          </Card>

          {currentTemp && (
            <Card title="积温指标" size="small" className={styles.accTempCard}>
              <Statistic
                title="当前日积温"
                value={currentTemp.accumulated}
                suffix="℃·d"
                valueStyle={{ color: currentTemp.accumulated > currentTemp.threshold ? '#ff4d4f' : '#52c41a' }}
              />
              <Progress
                percent={Math.min((currentTemp.accumulated / currentTemp.threshold) * 100, 100)}
                status={currentTemp.accumulated > currentTemp.threshold ? 'exception' : 'success'}
                format={(p) => `${p?.toFixed(0)}%`}
              />
              <Text type="secondary">阈值: {currentTemp.threshold}℃·d</Text>
            </Card>
          )}
        </Col>

        <Col span={16}>
          {selectedNode ? (
            <Row gutter={16}>
              <Col span={selectedNode.hasIrrigation || selectedNode.hasSideVent || selectedNode.hasRoofVent ? 12 : 24}>
                {zone && (
                  <ComfortIndicator
                    config={zone.comfortConfig}
                    values={{
                      airTemp: 24 + Math.random() * 4,
                      airHumidity: 65 + Math.random() * 15,
                      soilTemp: 20 + Math.random() * 3,
                      soilMoisture: 55 + Math.random() * 15,
                      ecValue: 2.0 + Math.random() * 1.0,
                    }}
                  />
                )}
              </Col>
              {(selectedNode.hasIrrigation || selectedNode.hasSideVent || selectedNode.hasRoofVent) && (
                <Col span={12}>
                  <ControlPanel node={selectedNode} />
                </Col>
              )}
            </Row>
          ) : (
            <Card>
              <Text type="secondary">请选择一个节点</Text>
            </Card>
          )}
        </Col>
      </Row>
    </div>
  );
};

export default ZoneDetail;