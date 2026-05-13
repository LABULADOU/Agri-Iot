import React, { useEffect, useState } from 'react';
import { Table, Card, Button, Space, Input, Modal, Form, Select, message, Popconfirm, Tag, Typography, Switch } from 'antd';
import { PlusOutlined, SearchOutlined, EditOutlined, DeleteOutlined } from '@ant-design/icons';
import { ruleApi } from '../../services/api';
import type { Rule } from '../../types';
import styles from './RuleList.module.css';

const { Title } = Typography;

const RuleList: React.FC = () => {
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
    { title: '触发类型', dataIndex: 'triggerType', key: 'triggerType', render: (t: string) => <Tag color={t === 'schedule' ? 'blue' : 'green'}>{t === 'schedule' ? '定时' : '条件'}</Tag> },
    { title: '状态', dataIndex: 'enabled', key: 'enabled', render: (enabled: boolean) => <Tag color={enabled ? 'success' : 'default'}>{enabled ? '启用' : '禁用'}</Tag> },
    { title: '创建时间', dataIndex: 'createdAt', key: 'createdAt', render: (t: string) => t ? new Date(t).toLocaleString('zh-CN') : '-' },
    {
      title: '操作',
      key: 'action',
      render: (_: unknown, record: Rule) => (
        <Space>
          <Switch size="small" checked={record.enabled} onChange={() => handleToggle(record)} />
          <Button type="link" icon={<EditOutlined />} onClick={() => handleEdit(record)}>编辑</Button>
          <Popconfirm title="确定删除?" onConfirm={() => handleDelete(record.id)}>
            <Button type="link" danger icon={<DeleteOutlined />}>删除</Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <Title level={4}>规则管理</Title>
        <Space>
          <Input placeholder="搜索规则" prefix={<SearchOutlined />} value={searchText} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setSearchText(e.target.value)} />
          <Button type="primary" icon={<PlusOutlined />} onClick={handleAdd}>新增规则</Button>
        </Space>
      </div>

      <Card>
        <Table columns={columns} dataSource={filteredRules} rowKey="id" loading={loading} pagination={{ pageSize: 10 }} />
      </Card>

      <Modal
        title={editingRule ? '编辑规则' : '新增规则'}
        open={modalVisible}
        onOk={handleSubmit}
        onCancel={() => setModalVisible(false)}
        width={600}
      >
        <Form form={form} layout="vertical">
          <Form.Item name="name" label="规则名称" rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="triggerType" label="触发类型" rules={[{ required: true }]}>
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
    </div>
  );
};

export default RuleList;