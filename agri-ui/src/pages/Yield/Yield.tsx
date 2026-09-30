import React, { useEffect, useState, useCallback } from 'react';
import {
  Typography, Table, Card, Row, Col, Button, Modal, Form, Input, InputNumber, DatePicker,
  Tag, Space, Popconfirm, Select, message, Statistic, Empty,
} from 'antd';
import { PlusOutlined, DeleteOutlined, EditOutlined } from '@ant-design/icons';
import dayjs from 'dayjs';
import { yieldApi, zoneApi } from '../../services/api';
import type { Harvest, YieldAnalysis } from '../../types';

const { Title, Text } = Typography;

const Yield: React.FC = () => {
  const [harvests, setHarvests] = useState<Harvest[]>([]);
  const [zones, setZones] = useState<Array<{ id: string; name: string }>>([]);
  const [analysis, setAnalysis] = useState<YieldAnalysis | null>(null);
  const [loading, setLoading] = useState(true);
  const [areaId, setAreaId] = useState<string>('');
  const [dateRange, setDateRange] = useState<[dayjs.Dayjs | null, dayjs.Dayjs | null]>([dayjs().subtract(90, 'day'), dayjs()]);

  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<Harvest | null>(null);
  const [form] = Form.useForm();

  const load = useCallback(async () => {
    try {
      const params: Record<string, string> = {};
      if (areaId) params.area_id = areaId;
      if (dateRange[0]) params.date_from = dateRange[0].format('YYYY-MM-DD');
      if (dateRange[1]) params.date_to = dateRange[1].format('YYYY-MM-DD');
      const [h, a] = await Promise.all([yieldApi.listHarvests(params), yieldApi.getAnalysis(params)]);
      setHarvests(h.harvests);
      setAnalysis(a);
    } catch {
      message.error('加载产量数据失败');
    } finally {
      setLoading(false);
    }
  }, [areaId, dateRange]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const params: Record<string, string> = {};
        if (areaId) params.area_id = areaId;
        if (dateRange[0]) params.date_from = dateRange[0].format('YYYY-MM-DD');
        if (dateRange[1]) params.date_to = dateRange[1].format('YYYY-MM-DD');
        const [h, a] = await Promise.all([yieldApi.listHarvests(params), yieldApi.getAnalysis(params)]);
        if (!cancelled) {
          setHarvests(h.harvests);
          setAnalysis(a);
        }
      } catch {
        if (!cancelled) message.error('加载产量数据失败');
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, [areaId, dateRange]);

  useEffect(() => {
    zoneApi.list().then(setZones).catch(() => {});
  }, []);

  const openCreate = () => {
    setEditing(null);
    form.resetFields();
    form.setFieldsValue({ area_id: areaId, harvest_date: dayjs() });
    setModalOpen(true);
  };

  const openEdit = (h: Harvest) => {
    setEditing(h);
    form.setFieldsValue({
      area_id: h.area_id, harvest_date: dayjs(h.harvest_date), quantity: h.quantity,
      unit: h.unit, grade: h.grade, price: h.price, operator: h.operator, notes: h.notes,
    });
    setModalOpen(true);
  };

  const handleSave = async () => {
    const v = await form.validateFields();
    const payload = {
      ...v,
      harvest_date: v.harvest_date.format('YYYY-MM-DD'),
    };
    try {
      if (editing) {
        await yieldApi.updateHarvest(editing.id, payload);
        message.success('已更新');
      } else {
        await yieldApi.createHarvest(payload);
        message.success('已记录采收');
      }
      setModalOpen(false);
      load();
    } catch {
      message.error('保存失败');
    }
  };

  const trend = analysis?.trend || [];
  const maxQty = Math.max(...trend.map(t => t.quantity), 1);

  const columns = [
    { title: '日期', dataIndex: 'harvest_date', key: 'date', width: 110 },
    { title: '区域', dataIndex: 'area_name', key: 'area', width: 110, render: (v: string) => v || '-' },
    { title: '作物', dataIndex: 'crop_name', key: 'crop', width: 100, render: (v?: string) => v || '-' },
    { title: '产量', dataIndex: 'quantity', key: 'qty', width: 90, render: (v: number, r: Harvest) => `${v} ${r.unit}` },
    { title: '等级', dataIndex: 'grade', key: 'grade', width: 70, render: (v: string) => v ? <Tag color="green">{v}</Tag> : '-' },
    { title: '单价', dataIndex: 'price', key: 'price', width: 80, render: (v: number) => v ? `¥${v}` : '-' },
    { title: '金额', dataIndex: 'amount', key: 'amount', width: 100, render: (v: number) => `¥${v.toFixed(2)}` },
    { title: '操作人', dataIndex: 'operator', key: 'op', width: 80, render: (v: string) => v || '-' },
    {
      title: '操作', key: 'actions', width: 110,
      render: (_: unknown, r: Harvest) => (
        <Space size={4}>
          <Button size="small" icon={<EditOutlined />} onClick={() => openEdit(r)} />
          <Popconfirm title="删除该采收记录？" onConfirm={async () => {
            await yieldApi.deleteHarvest(r.id);
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
      <Title level={4} style={{ margin: 0, marginBottom: 16 }}>📈 产量与收益</Title>

      <Row gutter={[16, 12]} style={{ marginBottom: 16 }}>
        <Col xs={12} sm={6}><Card><Statistic title="总产量" value={analysis?.yield.total_quantity || 0} precision={1} suffix="kg" /></Card></Col>
        <Col xs={12} sm={6}><Card><Statistic title="总收入" value={analysis?.yield.total_revenue || 0} precision={2} prefix="¥" /></Card></Col>
        <Col xs={12} sm={6}><Card><Statistic title="投入品成本(估)" value={analysis?.yield.input_cost_estimate || 0} precision={2} prefix="¥" valueStyle={{ color: '#cf1322' }} /></Card></Col>
        <Col xs={12} sm={6}>
          <Card>
            <Statistic
              title="净利润(估)"
              value={analysis?.yield.net_profit_estimate || 0}
              precision={2}
              prefix="¥"
              valueStyle={{ color: (analysis?.yield.net_profit_estimate || 0) >= 0 ? '#389e0d' : '#cf1322' }}
            />
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 12]} style={{ marginBottom: 16 }}>
        <Col xs={24} md={10}>
          <Card title="产量趋势（按日）" size="small" styles={{ body: { maxHeight: 200, overflowY: 'auto' } }}>
            {trend.length === 0 ? <Empty description="暂无数据" /> : (
              <div>
                {trend.map(t => (
                  <div key={t.date} style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                    <Text style={{ width: 90, fontSize: 12 }}>{t.date}</Text>
                    <div style={{ flex: 1, background: '#f0f0f0', borderRadius: 3, height: 18 }}>
                      <div style={{
                        width: `${(t.quantity / maxQty) * 100}%`, background: '#52c41a',
                        height: '100%', borderRadius: 3, minWidth: t.quantity > 0 ? 4 : 0,
                      }} />
                    </div>
                    <Text style={{ width: 70, fontSize: 12, textAlign: 'right' }}>{t.quantity}kg</Text>
                  </div>
                ))}
              </div>
            )}
          </Card>
        </Col>
        <Col xs={24} md={7}>
          <Card title="区域汇总" size="small" styles={{ body: { maxHeight: 200, overflowY: 'auto' } }}>
            {(analysis?.areas || []).map(a => (
              <div key={a.area_id} style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 6, fontSize: 13 }}>
                <span>{a.area_name || '未分配'}</span>
                <span>{a.total_quantity}kg · ¥{a.total_amount.toFixed(2)}</span>
              </div>
            ))}
          </Card>
        </Col>
        <Col xs={24} md={7}>
          <Card title="成本构成（农事操作）" size="small" styles={{ body: { maxHeight: 200, overflowY: 'auto' } }}>
            {Object.entries(analysis?.operations || {}).map(([k, v]) => (
              <div key={k} style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 6, fontSize: 13 }}>
                <span>{k}</span><span>{v} 次</span>
              </div>
            ))}
            {Object.keys(analysis?.operations || {}).length === 0 && <Empty description="本周期暂无施肥/打药记录" />}
          </Card>
        </Col>
      </Row>

      <Card
        title="采收记录"
        extra={
          <Space wrap style={{ justifyContent: 'flex-end' }}>
            <Select
              placeholder="区域" allowClear style={{ width: 120 }}
              value={areaId || undefined}
              onChange={v => setAreaId(v || '')}
              onClear={() => setAreaId('')}
              options={zones.map(z => ({ value: z.id, label: z.name }))}
            />
            <DatePicker.RangePicker value={dateRange} onChange={v => setDateRange(v || [null, null])} />
            <Button type="primary" icon={<PlusOutlined />} onClick={openCreate}>记录采收</Button>
          </Space>
        }
      >
        <Table rowKey="id" columns={columns} dataSource={harvests} loading={loading} pagination={{ pageSize: 10 }} size="middle" scroll={{ x: 'max-content' }} />
      </Card>

      <Modal
        title={editing ? '编辑采收记录' : '记录采收'}
        open={modalOpen}
        onOk={handleSave}
        onCancel={() => setModalOpen(false)}
      >
        <Form form={form} layout="vertical">
          <Form.Item name="area_id" label="区域" rules={[{ required: true }]}>
            <Select options={zones.map(z => ({ value: z.id, label: z.name }))} />
          </Form.Item>
          <Form.Item name="harvest_date" label="采收日期" rules={[{ required: true }]}>
            <DatePicker style={{ width: '100%' }} />
          </Form.Item>
          <Row gutter={12}>
            <Col xs={24} sm={8}><Form.Item name="quantity" label="产量" rules={[{ required: true }]}><InputNumber style={{ width: '100%' }} min={0} /></Form.Item></Col>
            <Col xs={12} sm={8}><Form.Item name="unit" label="单位"><Input defaultValue="kg" /></Form.Item></Col>
            <Col xs={12} sm={8}><Form.Item name="grade" label="等级"><Select allowClear options={[{ value: 'A' }, { value: 'B' }, { value: 'C' }]} /></Form.Item></Col>
          </Row>
          <Row gutter={12}>
            <Col xs={24} sm={12}><Form.Item name="price" label="单价（元/单位）"><InputNumber style={{ width: '100%' }} min={0} /></Form.Item></Col>
            <Col xs={24} sm={12}><Form.Item name="operator" label="操作人"><Input /></Form.Item></Col>
          </Row>
          <Form.Item name="notes" label="备注"><Input.TextArea rows={2} /></Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default Yield;