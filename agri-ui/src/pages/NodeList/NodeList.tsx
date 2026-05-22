import React, { useEffect, useState } from 'react';
import { Table, Button, Space, Input, Modal, Form, Select, message, Popconfirm, Tag, Typography, Row, Col } from 'antd';
import { PlusOutlined, SearchOutlined, EditOutlined, DeleteOutlined } from '@ant-design/icons';
import { nodeApi } from '../../services/api';
import type { SensorNode } from '../../types';
import styles from './NodeList.module.css';

const { Title } = Typography;

const NodeList: React.FC = () => {
  const [nodes, setNodes] = useState<SensorNode[]>([]);
  const [loading, setLoading] = useState(false);
  const [searchText, setSearchText] = useState('');
  const [modalVisible, setModalVisible] = useState(false);
  const [editingNode, setEditingNode] = useState<SensorNode | null>(null);
  const [form] = Form.useForm();

  useEffect(() => {
    fetchNodes();
  }, []);

  const fetchNodes = async () => {
    setLoading(true);
    try {
      const data = await nodeApi.list();
      setNodes(data);
    } catch {
      message.error('获取节点列表失败');
    } finally {
      setLoading(false);
    }
  };

  const handleAdd = () => {
    setEditingNode(null);
    form.resetFields();
    setModalVisible(true);
  };

  const handleEdit = (node: SensorNode) => {
    setEditingNode(node);
    form.setFieldsValue({ ...node });
    setModalVisible(true);
  };

  const handleDelete = async (id: string) => {
    try {
      await nodeApi.delete(id);
      message.success('删除成功');
      fetchNodes();
    } catch {
      message.error('删除失败');
    }
  };

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      if (editingNode) {
        await nodeApi.update(editingNode.id, values);
        message.success('更新成功');
      } else {
        await nodeApi.create(values);
        message.success('创建成功');
      }
      setModalVisible(false);
      fetchNodes();
    } catch (err) {
      console.error(err);
    }
  };

  const filteredNodes = nodes.filter(node =>
    node.name.includes(searchText) || node.id.includes(searchText)
  );

  const columns = [
    { title: '节点名称', dataIndex: 'name', key: 'name' },
    { title: '节点ID', dataIndex: 'id', key: 'id', width: 200 },
    { title: '状态', dataIndex: 'status', key: 'status', render: (s: string) => <Tag color={s === 'online' ? 'success' : 'default'}>{s === 'online' ? '在线' : '离线'}</Tag> },
    { title: '控制功能', key: 'controls', render: (_: unknown, record: SensorNode) => {
      const caps = record.capabilities ?? [];
      return (
        <Space size={4}>
          {caps.includes('actuator') && <Tag color="blue">执行器</Tag>}
          {caps.includes('sensor') && <Tag color="green">传感器</Tag>}
          {caps.length === 0 && <Tag>无</Tag>}
        </Space>
      );
    }},
    { title: '最后在线', key: 'lastSeen', render: (_: unknown, record: SensorNode) => record.updated_at ? new Date(Number(record.updated_at) * 1000).toLocaleString('zh-CN') : '-' },
    {
      title: '操作',
      key: 'action',
      render: (_: unknown, record: SensorNode) => (
        <Space>
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
        <Title level={4}>采集节点管理</Title>
        <Space>
          <Input placeholder="搜索节点" prefix={<SearchOutlined />} value={searchText} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setSearchText(e.target.value)} />
          <Button type="primary" icon={<PlusOutlined />} onClick={handleAdd}>新增节点</Button>
        </Space>
      </div>

      <Table columns={columns} dataSource={filteredNodes} rowKey="id" loading={loading} pagination={{ pageSize: 10 }} />

      <Modal
        title={editingNode ? '编辑节点' : '新增节点'}
        open={modalVisible}
        onOk={handleSubmit}
        onCancel={() => setModalVisible(false)}
        width={600}
      >
        <Form form={form} layout="vertical">
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="name" label="节点名称" rules={[{ required: true }]}>
                <Input />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="zoneId" label="所属区域" rules={[{ required: true }]}>
                <Select>
                  <Select.Option value="1">A区 - 番茄大棚</Select.Option>
                  <Select.Option value="2">B区 - 黄瓜大棚</Select.Option>
                  <Select.Option value="3">C区 - 草莓温室</Select.Option>
                </Select>
              </Form.Item>
            </Col>
          </Row>
          <Row gutter={16}>
            <Col span={24}>
              <Form.Item name="capabilities" label="能力">
                <Select mode="multiple" placeholder="选择设备能力">
                  <Select.Option value="sensor">传感器</Select.Option>
                  <Select.Option value="actuator">执行器</Select.Option>
                </Select>
              </Form.Item>
            </Col>
          </Row>
        </Form>
      </Modal>
    </div>
  );
};

export default NodeList;