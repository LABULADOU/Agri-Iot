import React, { useEffect, useState } from 'react';
import { Card, Tabs, Table, Button, Space, Modal, Form, Input, InputNumber, message, Typography, Row, Col, Select } from 'antd';
import { EditOutlined, SettingOutlined } from '@ant-design/icons';
import { zoneApi } from '../../services/api';
import type { Zone } from '../../types';
import styles from './Settings.module.css';

const { Title, Text } = Typography;

const Settings: React.FC = () => {
  const [zones, setZones] = useState<Zone[]>([]);
  const [loading, setLoading] = useState(false);
  const [modalVisible, setModalVisible] = useState(false);
  const [editingZone, setEditingZone] = useState<Zone | null>(null);
  const [form] = Form.useForm();

  useEffect(() => {
    fetchZones();
  }, []);

  const fetchZones = async () => {
    setLoading(true);
    try {
      const data = await zoneApi.list();
      setZones(data);
    } catch {
      message.error('获取配置失败');
    } finally {
      setLoading(false);
    }
  };

  const handleEdit = (zone: Zone) => {
    setEditingZone(zone);
    const cc = zone.comfortConfig ?? { airTemp: { min: 18, max: 28 }, airHumidity: { min: 60, max: 80 }, soilTemp: { min: 15, max: 25 }, soilMoisture: { min: 40, max: 70 }, ecValue: { min: 1.5, max: 3.5 } };
    form.setFieldsValue({
      ...zone,
      comfortConfig: {
        airTemp: { min: cc.airTemp.min, max: cc.airTemp.max },
        airHumidity: { min: cc.airHumidity.min, max: cc.airHumidity.max },
        soilTemp: { min: cc.soilTemp.min, max: cc.soilTemp.max },
        soilMoisture: { min: cc.soilMoisture.min, max: cc.soilMoisture.max },
        ecValue: { min: cc.ecValue.min, max: cc.ecValue.max },
      },
    });
    setModalVisible(true);
  };

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      await zoneApi.update(editingZone!.id, values);
      message.success('保存成功');
      setModalVisible(false);
      fetchZones();
    } catch (err) {
      console.error(err);
    }
  };

  const columns = [
    { title: '区域名称', dataIndex: 'name', key: 'name' },
    { title: '作物类型', key: 'cropType', render: (_: unknown, record: Zone) => record.cropType ?? '--' },
    { title: '空气温度', key: 'airTemp', render: (_: unknown, record: Zone) => record.comfortConfig ? `${record.comfortConfig.airTemp.min} ~ ${record.comfortConfig.airTemp.max}℃` : '--' },
    { title: '空气湿度', key: 'airHumidity', render: (_: unknown, record: Zone) => record.comfortConfig ? `${record.comfortConfig.airHumidity.min} ~ ${record.comfortConfig.airHumidity.max}%` : '--' },
    { title: '土壤温度', key: 'soilTemp', render: (_: unknown, record: Zone) => record.comfortConfig ? `${record.comfortConfig.soilTemp.min} ~ ${record.comfortConfig.soilTemp.max}℃` : '--' },
    { title: '土壤湿度', key: 'soilMoisture', render: (_: unknown, record: Zone) => record.comfortConfig ? `${record.comfortConfig.soilMoisture.min} ~ ${record.comfortConfig.soilMoisture.max}%` : '--' },
    { title: 'EC值', key: 'ecValue', render: (_: unknown, record: Zone) => record.comfortConfig ? `${record.comfortConfig.ecValue.min} ~ ${record.comfortConfig.ecValue.max} dS/m` : '--' },
    {
      title: '操作',
      key: 'action',
      render: (_: unknown, record: Zone) => (
        <Button type="link" icon={<EditOutlined />} onClick={() => handleEdit(record)}>配置</Button>
      ),
    },
  ];

  const comfortItem = (prefix: string, unit: string) => (
    <Row gutter={8}>
      <Col span={12}>
        <Form.Item name={['comfortConfig', prefix, 'min']} label="最小值" rules={[{ required: true }]}>
          <InputNumber style={{ width: '100%' }} addonAfter={unit} />
        </Form.Item>
      </Col>
      <Col span={12}>
        <Form.Item name={['comfortConfig', prefix, 'max']} label="最大值" rules={[{ required: true }]}>
          <InputNumber style={{ width: '100%' }} addonAfter={unit} />
        </Form.Item>
      </Col>
    </Row>
  );

  return (
    <div className={styles.container}>
      <Title level={4}>系统设置</Title>

      <Tabs
        items={[
          {
            key: 'comfort',
            label: '舒适区间配置',
            children: (
              <Card
                title={<Space><SettingOutlined />作物舒适区间设置</Space>}
              >
                <Table columns={columns} dataSource={zones} rowKey="id" loading={loading} pagination={false} />
              </Card>
            ),
          },
          {
            key: 'system',
            label: '系统配置',
            children: (
              <Card title="系统参数">
                <Form layout="vertical">
                  <Row gutter={16}>
                    <Col span={12}>
                      <Form.Item label="数据采集间隔">
                        <Select defaultValue="60">
                          <Select.Option value="30">30秒</Select.Option>
                          <Select.Option value="60">1分钟</Select.Option>
                          <Select.Option value="300">5分钟</Select.Option>
                        </Select>
                      </Form.Item>
                    </Col>
                    <Col span={12}>
                      <Form.Item label="积温计算阈值">
                        <InputNumber defaultValue={10} addonAfter="℃" style={{ width: '100%' }} />
                      </Form.Item>
                    </Col>
                  </Row>
                  <Row gutter={16}>
                    <Col span={12}>
                      <Form.Item label="预警温度偏差">
                        <InputNumber defaultValue={5} addonAfter="℃" style={{ width: '100%' }} />
                      </Form.Item>
                    </Col>
                    <Col span={12}>
                      <Form.Item label="和风天气Key">
                        <Input placeholder="请输入API Key" />
                      </Form.Item>
                    </Col>
                  </Row>
                  <Button type="primary">保存配置</Button>
                </Form>
              </Card>
            ),
          },
        ]}
      />

      <Modal
        title="舒适区间配置"
        open={modalVisible}
        onOk={handleSubmit}
        onCancel={() => setModalVisible(false)}
        width={700}
      >
        <Form form={form} layout="vertical">
          <Title level={5}>区域信息</Title>
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="name" label="区域名称" rules={[{ required: true }]}>
                <Input />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="cropType" label="作物类型" rules={[{ required: true }]}>
                <Input />
              </Form.Item>
            </Col>
          </Row>

          <Title level={5}>舒适区间阈值</Title>
          <Text type="secondary" className={styles.hint}>设置各指标的最佳生长范围，超出范围将触发预警</Text>

          <div className={styles.comfortSection}>
            <Text strong>空气温度</Text>
            {comfortItem('airTemp', '℃')}
          </div>
          <div className={styles.comfortSection}>
            <Text strong>空气湿度</Text>
            {comfortItem('airHumidity', '%')}
          </div>
          <div className={styles.comfortSection}>
            <Text strong>土壤温度</Text>
            {comfortItem('soilTemp', '℃')}
          </div>
          <div className={styles.comfortSection}>
            <Text strong>土壤湿度</Text>
            {comfortItem('soilMoisture', '%')}
          </div>
          <div className={styles.comfortSection}>
            <Text strong>EC值</Text>
            {comfortItem('ecValue', 'dS/m')}
          </div>
        </Form>
      </Modal>
    </div>
  );
};

export default Settings;