import React, { useEffect, useState } from 'react';
import { Table, Card, Button, Space, Input, Modal, Form, Select, message, Popconfirm, Typography } from 'antd';
import { PlusOutlined, SearchOutlined, EditOutlined, DeleteOutlined, EyeOutlined } from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
import { zoneApi } from '../../services/api';
import type { Zone } from '../../types';
import styles from './ZoneList.module.css';

const { Title } = Typography;

const ZoneList: React.FC = () => {
  const navigate = useNavigate();
  const [zones, setZones] = useState<Zone[]>([]);
  const [loading, setLoading] = useState(false);
  const [searchText, setSearchText] = useState('');
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
      message.error('获取区域列表失败');
    } finally {
      setLoading(false);
    }
  };

  const handleAdd = () => {
    setEditingZone(null);
    form.resetFields();
    setModalVisible(true);
  };

  const handleEdit = (zone: Zone) => {
    setEditingZone(zone);
    form.setFieldsValue({
      ...zone,
      comfortConfig: zone.comfortConfig,
    });
    setModalVisible(true);
  };

  const handleDelete = async (id: string) => {
    try {
      await zoneApi.delete(id);
      message.success('删除成功');
      fetchZones();
    } catch {
      message.error('删除失败');
    }
  };

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      if (editingZone) {
        await zoneApi.update(editingZone.id, values);
        message.success('更新成功');
      } else {
        await zoneApi.create(values);
        message.success('创建成功');
      }
      setModalVisible(false);
      fetchZones();
    } catch (err) {
      console.error(err);
    }
  };

  const filteredZones = zones.filter(zone =>
    zone.name.includes(searchText) || zone.location.includes(searchText)
  );

  const columns = [
    { title: '区域名称', dataIndex: 'name', key: 'name' },
    { title: '位置', dataIndex: 'location', key: 'location' },
    { title: '作物类型', dataIndex: 'cropType', key: 'cropType' },
    {
      title: '节点数量',
      dataIndex: 'nodeIds',
      key: 'nodeCount',
      render: (nodeIds: string[]) => nodeIds.length,
    },
    {
      title: '操作',
      key: 'action',
      render: (_: unknown, record: Zone) => (
        <Space>
          <Button type="link" icon={<EyeOutlined />} onClick={() => navigate(`/zones/${record.id}`)}>查看</Button>
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
        <Title level={4}>区域管理</Title>
        <Space>
          <Input placeholder="搜索区域" prefix={<SearchOutlined />} value={searchText} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setSearchText(e.target.value)} />
          <Button type="primary" icon={<PlusOutlined />} onClick={handleAdd}>新增区域</Button>
        </Space>
      </div>

      <Card>
        <Table columns={columns} dataSource={filteredZones} rowKey="id" loading={loading} pagination={{ pageSize: 10 }} />
      </Card>

      <Modal
        title={editingZone ? '编辑区域' : '新增区域'}
        open={modalVisible}
        onOk={handleSubmit}
        onCancel={() => setModalVisible(false)}
        width={600}
      >
        <Form form={form} layout="vertical">
          <Form.Item name="name" label="区域名称" rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="location" label="位置" rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="cropType" label="作物类型" rules={[{ required: true }]}>
            <Select>
              <Select.Option value="番茄">番茄</Select.Option>
              <Select.Option value="黄瓜">黄瓜</Select.Option>
              <Select.Option value="草莓">草莓</Select.Option>
              <Select.Option value="辣椒">辣椒</Select.Option>
              <Select.Option value="茄子">茄子</Select.Option>
              <Select.Option value="叶菜">叶菜</Select.Option>
            </Select>
          </Form.Item>
          <Form.Item name="description" label="描述">
            <Input.TextArea rows={2} />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default ZoneList;