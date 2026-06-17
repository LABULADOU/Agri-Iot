import React, { useEffect, useState } from 'react';
import { Card, Tabs, Table, Button, Space, Modal, Form, Input, InputNumber, message, Typography, Row, Col, Select, Popconfirm, Switch, Tag } from 'antd';
import { EditOutlined, SettingOutlined, PlusOutlined, SearchOutlined, DeleteOutlined } from '@ant-design/icons';
import { useSearchParams } from 'react-router-dom';
import { zoneApi, ruleApi } from '../../services/api';
import type { Zone, Rule } from '../../types';
import styles from './Settings.module.css';

const { Title, Text } = Typography;

const ComfortTab: React.FC = () => {
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
    { title: '作物类型', key: 'cropType', responsive: ['md'] as ['md'], render: (_: unknown, record: Zone) => record.cropType ?? '--' },
    { title: '空气温度', key: 'airTemp', render: (_: unknown, record: Zone) => record.comfortConfig ? `${record.comfortConfig.airTemp.min} ~ ${record.comfortConfig.airTemp.max}℃` : '--' },
    { title: '空气湿度', key: 'airHumidity', render: (_: unknown, record: Zone) => record.comfortConfig ? `${record.comfortConfig.airHumidity.min} ~ ${record.comfortConfig.airHumidity.max}%` : '--' },
    { title: '土壤温度', key: 'soilTemp', responsive: ['md'] as ['md'], render: (_: unknown, record: Zone) => record.comfortConfig ? `${record.comfortConfig.soilTemp.min} ~ ${record.comfortConfig.soilTemp.max}℃` : '--' },
    { title: '土壤湿度', key: 'soilMoisture', responsive: ['md'] as ['md'], render: (_: unknown, record: Zone) => record.comfortConfig ? `${record.comfortConfig.soilMoisture.min} ~ ${record.comfortConfig.soilMoisture.max}%` : '--' },
    { title: 'EC值', key: 'ecValue', responsive: ['md'] as ['md'], render: (_: unknown, record: Zone) => record.comfortConfig ? `${record.comfortConfig.ecValue.min} ~ ${record.comfortConfig.ecValue.max} dS/m` : '--' },
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
    <>
      <Card title={<Space><SettingOutlined />作物舒适区间设置</Space>}>
        <Table columns={columns} dataSource={zones} rowKey="id" loading={loading} size="small" pagination={false} scroll={{ x: 'max-content' }} />
      </Card>

      <Modal
        title="舒适区间配置"
        open={modalVisible}
        onOk={handleSubmit}
        onCancel={() => setModalVisible(false)}
        width={700}
        className={styles.settingsModal}
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
    </>
  );
};

const RulesTab: React.FC = () => {
  const [rules, setRules] = useState<Rule[]>([]);
  const [loading, setLoading] = useState(false);
  const [searchText, setSearchText] = useState('');
  const [modalVisible, setModalVisible] = useState(false);
  const [editingRule, setEditingRule] = useState<Rule | null>(null);
  const [form] = Form.useForm();

  useEffect(() => {
    fetchRules();
  }, []);

  const fetchRules = async () => {
    setLoading(true);
    try {
      const data = await ruleApi.list();
      setRules(data);
    } catch {
      message.error('获取规则列表失败');
    } finally {
      setLoading(false);
    }
  };

  const handleAdd = () => {
    setEditingRule(null);
    form.resetFields();
    setModalVisible(true);
  };

  const handleEdit = (rule: Rule) => {
    setEditingRule(rule);
    form.setFieldsValue(rule);
    setModalVisible(true);
  };

  const handleDelete = async (id: string) => {
    try {
      await ruleApi.delete(id);
      message.success('删除成功');
      fetchRules();
    } catch {
      message.error('删除失败');
    }
  };

  const handleToggle = async (rule: Rule) => {
    try {
      await ruleApi.update(rule.id, { enabled: !rule.enabled });
      message.success(rule.enabled ? '规则已暂停' : '规则已启用');
      fetchRules();
    } catch {
      message.error('操作失败');
    }
  };

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      if (editingRule) {
        await ruleApi.update(editingRule.id, values);
        message.success('更新成功');
      } else {
        await ruleApi.create(values);
        message.success('创建成功');
      }
      setModalVisible(false);
      fetchRules();
    } catch (err) {
      console.error(err);
    }
  };

  const filteredRules = rules.filter(rule =>
    rule.name.includes(searchText)
  );

  const columns = [
    { title: '规则名称', dataIndex: 'name', key: 'name' },
    { title: '触发类型', key: 'triggerType', responsive: ['md'] as ['md'], render: (_: unknown, record: Rule) => {
      const t = record.triggerType || record.trigger_type;
      return <Tag color={t === 'schedule' ? 'blue' : 'green'}>{t === 'schedule' ? '定时' : '条件'}</Tag>;
    }},
    { title: '状态', dataIndex: 'enabled', key: 'enabled', render: (enabled: boolean) => <Tag color={enabled ? 'success' : 'default'}>{enabled ? '启用' : '禁用'}</Tag> },
    { title: '创建时间', key: 'createdAt', responsive: ['md'] as ['md'], render: (_: unknown, record: Rule) => {
      const t = record.createdAt || record.created_at;
      return t ? new Date(t).toLocaleString('zh-CN') : '-';
    }},
    {
      title: '操作',
      key: 'action',
      render: (_: unknown, record: Rule) => (
        <Space>
          <Switch checked={record.enabled} onChange={() => handleToggle(record)} />
          <Button type="link" icon={<EditOutlined />} onClick={() => handleEdit(record)}>编辑</Button>
          <Popconfirm title="确定删除?" onConfirm={() => handleDelete(record.id)}>
            <Button type="link" danger icon={<DeleteOutlined />}>删除</Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <>
      <div className={styles.rulesHeader}>
        <Space>
          <Input placeholder="搜索规则" prefix={<SearchOutlined />} value={searchText} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setSearchText(e.target.value)} />
          <Button type="primary" icon={<PlusOutlined />} onClick={handleAdd}>新增规则</Button>
        </Space>
      </div>

      <Table columns={columns} dataSource={filteredRules} rowKey="id" loading={loading} size="small" pagination={{ pageSize: 10 }} scroll={{ x: 'max-content' }} />

      <Modal
        title={editingRule ? '编辑规则' : '新增规则'}
        open={modalVisible}
        onOk={handleSubmit}
        onCancel={() => setModalVisible(false)}
        width={600}
        className={styles.ruleModal}
      >
        <Form form={form} layout="vertical">
          <Form.Item name="name" label="规则名称" rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="trigger_type" label="触发类型" rules={[{ required: true }]}>
            <Select>
              <Select.Option value="schedule">定时触发</Select.Option>
              <Select.Option value="condition">条件触发</Select.Option>
            </Select>
          </Form.Item>
          <Form.Item name="schedule" label="执行时间" help="定时触发时使用，如: 0 8 * * * (每天8点)">
            <Input />
          </Form.Item>
          <Form.Item name="enabled" label="启用状态" valuePropName="checked" initialValue={true}>
            <Switch />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
};

const SystemTab: React.FC = () => (
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
      <Button type="primary" onClick={() => message.info('系统配置功能开发中')}>保存配置</Button>
    </Form>
  </Card>
);

const Settings: React.FC = () => {
  const [searchParams, setSearchParams] = useSearchParams();
  const tab = searchParams.get('tab') || 'comfort';

  return (
    <div className={styles.container}>
      <Title level={4}>系统设置</Title>
      <Tabs
        activeKey={tab}
        onChange={(key) => setSearchParams({ tab: key })}
        items={[
          { key: 'comfort', label: '舒适区间', children: <ComfortTab /> },
          { key: 'rules', label: '自动化规则', children: <RulesTab /> },
          { key: 'system', label: '系统配置', children: <SystemTab /> },
        ]}
      />
    </div>
  );
};

export default Settings;
