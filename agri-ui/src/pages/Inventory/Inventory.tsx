import React, { useEffect, useState, useCallback } from 'react';
import {
  Typography, Table, Card, Row, Col, Button, Modal, Form, Input, InputNumber, Select,
  Tag, Space, Popconfirm, Drawer, message, Statistic, Tooltip,
} from 'antd';
import {
  PlusOutlined, DeleteOutlined, EditOutlined, InboxOutlined, ExportOutlined,
  ReloadOutlined, HistoryOutlined,
} from '@ant-design/icons';
import { inventoryApi } from '../../services/api';
import type { InventoryItem, InventoryCategory, InventoryTransaction } from '../../types';

const { Title } = Typography;

const CATEGORY_LABEL: Record<InventoryCategory, string> = {
  seed: '种子',
  fertilizer: '肥料',
  pesticide: '农药',
  materiel: '物料',
  other: '其他',
};

const CATEGORY_COLOR: Record<InventoryCategory, string> = {
  seed: 'green',
  fertilizer: 'blue',
  pesticide: 'red',
  materiel: 'orange',
  other: 'default',
};

interface StatCards {
  total_items: number;
  total_value: number;
  low_stock_count: number;
}

const Inventory: React.FC = () => {
  const [items, setItems] = useState<InventoryItem[]>([]);
  const [summary, setSummary] = useState<StatCards>({ total_items: 0, total_value: 0, low_stock_count: 0 });
  const [loading, setLoading] = useState(true);
  const [categoryFilter, setCategoryFilter] = useState<string>('');
  const [search, setSearch] = useState('');
  const [lowOnly, setLowOnly] = useState(false);

  const [editItem, setEditItem] = useState<InventoryItem | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [form] = Form.useForm();

  const [txnDrawer, setTxnDrawer] = useState<InventoryItem | null>(null);
  const [transactions, setTransactions] = useState<InventoryTransaction[]>([]);
  const [txnModal, setTxnModal] = useState<{ item: InventoryItem; type: 'in' | 'out' | 'adjust' } | null>(null);
  const [txnForm] = Form.useForm();

  const load = useCallback(async () => {
    try {
      const params: Record<string, string | boolean> = {};
      if (categoryFilter) params.category = categoryFilter;
      if (search) params.search = search;
      if (lowOnly) params.low_stock = true;
      const [itemData, summaryData] = await Promise.all([
        inventoryApi.listItems(params),
        inventoryApi.getSummary(),
      ]);
      setItems(itemData.items);
      setSummary(summaryData.summary);
    } catch {
      message.error('加载库存失败');
    } finally {
      setLoading(false);
    }
  }, [categoryFilter, search, lowOnly]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoading(true);
      const params: Record<string, string | boolean> = {};
      if (categoryFilter) params.category = categoryFilter;
      if (search) params.search = search;
      if (lowOnly) params.low_stock = true;
      try {
        const [itemData, summaryData] = await Promise.all([
          inventoryApi.listItems(params),
          inventoryApi.getSummary(),
        ]);
        if (cancelled) return;
        setItems(itemData.items);
        setSummary(summaryData.summary);
      } catch {
        if (!cancelled) message.error('加载库存失败');
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, [categoryFilter, search, lowOnly]);

  const openCreate = () => {
    setEditItem(null);
    form.resetFields();
    setModalOpen(true);
  };

  const openEdit = (item: InventoryItem) => {
    setEditItem(item);
    form.setFieldsValue({
      name: item.name, category: item.category, unit: item.unit, price: item.price,
      warning_threshold: item.warning_threshold, batch_no: item.batch_no,
      expiry_date: item.expiry_date, manufacturer: item.manufacturer,
      specs: item.specs, notes: item.notes,
    });
    setModalOpen(true);
  };

  const handleSave = async () => {
    const values = await form.validateFields();
    try {
      if (editItem) {
        await inventoryApi.updateItem(editItem.id, values);
        message.success('已更新');
      } else {
        await inventoryApi.createItem({ ...values, stock: values.stock || 0 });
        message.success('已创建');
      }
      setModalOpen(false);
      load();
    } catch {
      message.error('保存失败');
    }
  };

  const openTxn = (item: InventoryItem, type: 'in' | 'out' | 'adjust') => {
    setTxnModal({ item, type });
    txnForm.resetFields();
    txnForm.setFieldsValue({ quantity: type === 'adjust' ? item.stock : undefined });
  };

  const handleTxn = async () => {
    if (!txnModal) return;
    const values = await txnForm.validateFields();
    try {
      const res = await inventoryApi.createTransaction({
        item_id: txnModal.item.id,
        txn_type: txnModal.type,
        quantity: values.quantity,
        operator: values.operator,
        note: values.note,
      });
      message.success(`操作成功，当前库存 ${res.new_stock}`);
      setTxnModal(null);
      load();
    } catch (e) {
      const err = e as { response?: { data?: { error?: string } } };
      message.error(err.response?.data?.error || '操作失败');
    }
  };

  const showTransactions = async (item: InventoryItem) => {
    setTxnDrawer(item);
    try {
      const data = await inventoryApi.listTransactions(item.id);
      setTransactions(data.transactions);
    } catch {
      message.error('加载流水失败');
    }
  };

  const columns = [
    { title: '名称', dataIndex: 'name', key: 'name', width: 180 },
    {
      title: '分类', dataIndex: 'category', key: 'category', width: 90,
      render: (v: InventoryCategory) => <Tag color={CATEGORY_COLOR[v]}>{CATEGORY_LABEL[v]}</Tag>,
    },
    {
      title: '库存', dataIndex: 'stock', key: 'stock', width: 100,
      render: (v: number, r: InventoryItem) => (
        <span style={{ color: r.low_stock ? '#cf1322' : 'inherit', fontWeight: r.low_stock ? 600 : 400 }}>
          {v} {r.unit}
        </span>
      ),
    },
    { title: '阈值', dataIndex: 'warning_threshold', key: 'wt', width: 70, render: (v: number) => v || '-' },
    { title: '单价(元)', dataIndex: 'price', key: 'price', width: 90, render: (v: number) => v || '-' },
    { title: '批次', dataIndex: 'batch_no', key: 'batch', width: 110, render: (v: string) => v || '-' },
    { title: '保质期', dataIndex: 'expiry_date', key: 'expiry', width: 100, render: (v: string) => v || '-' },
    { title: '厂家', dataIndex: 'manufacturer', key: 'mfr', width: 120, render: (v: string) => v || '-' },
    {
      title: '操作', key: 'actions', width: 260,
      render: (_: unknown, r: InventoryItem) => (
        <Space size={4}>
          <Tooltip title="入库"><Button size="small" icon={<InboxOutlined />} onClick={() => openTxn(r, 'in')} /></Tooltip>
          <Tooltip title="出库"><Button size="small" icon={<ExportOutlined />} onClick={() => openTxn(r, 'out')} /></Tooltip>
          <Tooltip title="盘点"><Button size="small" icon={<ReloadOutlined />} onClick={() => openTxn(r, 'adjust')} /></Tooltip>
          <Tooltip title="流水"><Button size="small" icon={<HistoryOutlined />} onClick={() => showTransactions(r)} /></Tooltip>
          <Tooltip title="编辑"><Button size="small" icon={<EditOutlined />} onClick={() => openEdit(r)} /></Tooltip>
          <Popconfirm title="删除该物品？" onConfirm={async () => {
            await inventoryApi.deleteItem(r.id);
            message.success('已删除');
            load();
          }} okText="删除" cancelText="取消">
            <Button size="small" danger icon={<DeleteOutlined />} />
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div style={{ padding: 16 }}>
      <Title level={4} style={{ margin: 0, marginBottom: 16 }}>📦 库存与投入品</Title>
      <Row gutter={[16, 12]} style={{ marginBottom: 16 }}>
        <Col xs={24} sm={8}><Card><Statistic title="投入品种类" value={summary.total_items} /></Card></Col>
        <Col xs={24} sm={8}><Card><Statistic title="库存总价值（元）" value={summary.total_value} precision={2} /></Card></Col>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic title="低库存预警" value={summary.low_stock_count} valueStyle={summary.low_stock_count > 0 ? { color: '#cf1322' } : {}} />
          </Card>
        </Col>
      </Row>

      <Card
        title="物品清单"
        extra={
          <Space wrap style={{ justifyContent: 'flex-end' }}>
            <Select
              placeholder="分类" allowClear style={{ width: 110 }}
              value={categoryFilter || undefined}
              onChange={v => setCategoryFilter(v || '')}
              onClear={() => setCategoryFilter('')}
              options={Object.entries(CATEGORY_LABEL).map(([value, label]) => ({ value, label }))}
            />
            <Input.Search placeholder="搜索名称/厂家/批次" style={{ width: 180 }} onSearch={setSearch} allowClear />
            <Button type={lowOnly ? 'primary' : 'default'} size="middle" onClick={() => setLowOnly(v => !v)}>
              仅看低库存
            </Button>
            <Button type="primary" icon={<PlusOutlined />} onClick={openCreate}>新增物品</Button>
          </Space>
        }
      >
        <Table rowKey="id" columns={columns} dataSource={items} loading={loading} pagination={{ pageSize: 12 }} size="middle" scroll={{ x: 'max-content' }} />
      </Card>

      <Modal
        title={editItem ? '编辑物品' : '新增物品'}
        open={modalOpen}
        onOk={handleSave}
        onCancel={() => setModalOpen(false)}
        width={560}
      >
        <Form form={form} layout="vertical">
          <Row gutter={12}>
            <Col xs={24} sm={14}>
              <Form.Item name="name" label="名称" rules={[{ required: true }]}>
                <Input placeholder="如：高氮型水溶肥(30-10-20)" />
              </Form.Item>
            </Col>
            <Col xs={24} sm={10}>
              <Form.Item name="category" label="分类" rules={[{ required: true }]}>
                <Select options={Object.entries(CATEGORY_LABEL).map(([value, label]) => ({ value, label }))} />
              </Form.Item>
            </Col>
          </Row>
          <Row gutter={12}>
            <Col xs={12} sm={8}><Form.Item name="unit" label="单位"><Input placeholder="kg/瓶/袋" /></Form.Item></Col>
            <Col xs={12} sm={8}><Form.Item name="price" label="单价（元）"><InputNumber style={{ width: '100%' }} min={0} /></Form.Item></Col>
            {!editItem && (
              <Col xs={24} sm={8}><Form.Item name="stock" label="初始库存"><InputNumber style={{ width: '100%' }} min={0} /></Form.Item></Col>
            )}
          </Row>
          <Row gutter={12}>
            <Col xs={24} sm={8}><Form.Item name="warning_threshold" label="低库存阈值"><InputNumber style={{ width: '100%' }} min={0} /></Form.Item></Col>
            <Col xs={24} sm={8}><Form.Item name="batch_no" label="批次号"><Input /></Form.Item></Col>
            <Col xs={24} sm={8}><Form.Item name="expiry_date" label="保质期"><Input placeholder="YYYY-MM-DD" /></Form.Item></Col>
          </Row>
          <Row gutter={12}>
            <Col xs={24} sm={12}><Form.Item name="manufacturer" label="厂家"><Input /></Form.Item></Col>
            <Col xs={24} sm={12}><Form.Item name="specs" label="规格"><Input placeholder="如：25kg/袋" /></Form.Item></Col>
          </Row>
          <Form.Item name="notes" label="备注"><Input.TextArea rows={2} /></Form.Item>
        </Form>
      </Modal>

      <Modal
        title={`${txnModal ? CATEGORY_LABEL[txnModal.item.category] + ' · ' + txnModal.item.name : ''} — ${{ in: '入库', out: '出库', adjust: '盘点' }[txnModal?.type || 'in']}`}
        open={!!txnModal}
        onOk={handleTxn}
        onCancel={() => setTxnModal(null)}
      >
        <Form form={txnForm} layout="vertical">
          <Form.Item
            name="quantity"
            label={txnModal?.type === 'adjust' ? '盘点后实际库存' : '数量'}
            rules={[{ required: true }]}
          >
            <InputNumber style={{ width: '100%' }} min={0} />
          </Form.Item>
          {txnModal?.type === 'out' && (
            <div style={{ color: '#8c8c8c', fontSize: 12, marginBottom: 12 }}>
              当前库存：{txnModal.item.stock} {txnModal.item.unit}，出库数量不能超过库存
            </div>
          )}
          <Form.Item name="operator" label="操作人"><Input /></Form.Item>
          <Form.Item name="note" label="备注"><Input.TextArea rows={2} /></Form.Item>
        </Form>
      </Modal>

      <Drawer
        title={`${txnDrawer?.name || ''} 出入库流水`}
        open={!!txnDrawer}
        onClose={() => setTxnDrawer(null)}
        width="min(520px, 94vw)"
      >
        <Table
          rowKey="id"
          size="small"
          dataSource={transactions}
          pagination={{ pageSize: 10 }}
          columns={[
            {
              title: '类型', dataIndex: 'txn_type', width: 80,
              render: (v: string) => {
                const color = v === 'in' ? 'green' : v === 'out' ? 'red' : 'orange';
                const label = v === 'in' ? '入库' : v === 'out' ? '出库' : '盘点';
                return <Tag color={color}>{label}</Tag>;
              },
            },
            { title: '数量', dataIndex: 'quantity', width: 100, render: (v: number) => (v > 0 ? `+${v}` : v) },
            { title: '经手人', dataIndex: 'operator', width: 80, render: (v: string) => v || '-' },
            { title: '备注', dataIndex: 'note', ellipsis: true },
            { title: '时间', dataIndex: 'created_at', width: 130, render: (v: number) => new Date(v * 1000).toLocaleString() },
          ]}
        />
      </Drawer>
    </div>
  );
};

export default Inventory;