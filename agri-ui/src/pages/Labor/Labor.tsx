import React, { useEffect, useState, useCallback } from 'react';
import {
  Typography, Table, Card, Row, Col, Button, Modal, Form, Input, InputNumber, DatePicker,
  Tag, Space, Popconfirm, Select, message, Statistic, Empty, Alert,
} from 'antd';
import { PlusOutlined, DeleteOutlined, EditOutlined, TeamOutlined, DollarOutlined, CalendarOutlined, UserOutlined } from '@ant-design/icons';
import dayjs from 'dayjs';
import { laborApi, zoneApi } from '../../services/api';
import type { LaborRecord, LaborSummary } from '../../types';

const { Title, Text } = Typography;

const LABOR_CATEGORIES = ['整地', '定植', '播种', '育苗', '打药', '施肥', '灌溉', '除草', '采收', '包装', '巡棚', '设备维护', '其他'];

const Labor: React.FC = () => {
  const [records, setRecords] = useState<LaborRecord[]>([]);
  const [zones, setZones] = useState<Array<{ id: string; name: string }>>([]);
  const [summary, setSummary] = useState<LaborSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [total, setTotal] = useState(0);
  const [areaId, setAreaId] = useState<string>('');
  const [dateRange, setDateRange] = useState<[dayjs.Dayjs | null, dayjs.Dayjs | null]>([dayjs().subtract(30, 'day'), dayjs()]);

  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<LaborRecord | null>(null);
  const [form] = Form.useForm();

  const buildParams = useCallback((page = 1) => {
    const params: Record<string, string> = { page: String(page), limit: '50' };
    if (areaId) params.area_id = areaId;
    if (dateRange[0]) params.date_from = dateRange[0].format('YYYY-MM-DD');
    if (dateRange[1]) params.date_to = dateRange[1].format('YYYY-MM-DD');
    return params;
  }, [areaId, dateRange]);

  const load = useCallback(async () => {
    try {
      const params = buildParams();
      const [r, s] = await Promise.all([laborApi.listRecords(params), laborApi.getSummary(params)]);
      setRecords(r.records);
      setTotal(r.total);
      setSummary(s.summary);
    } catch {
      message.error('加载用工数据失败');
    } finally {
      setLoading(false);
    }
  }, [buildParams]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const params = buildParams();
        const [r, s] = await Promise.all([laborApi.listRecords(params), laborApi.getSummary(params)]);
        if (!cancelled) {
          setRecords(r.records);
          setTotal(r.total);
          setSummary(s.summary);
        }
      } catch {
        if (!cancelled) message.error('加载用工数据失败');
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, [buildParams]);

  useEffect(() => {
    zoneApi.list().then(setZones).catch(() => {});
  }, []);

  const updateAmount = () => {
    const wc = form.getFieldValue('worker_count') || 1;
    const wh = form.getFieldValue('work_hours') || 0;
    const rate = form.getFieldValue('rate') || 0;
    form.setFieldsValue({ amount_preview: wh * wc * rate });
  };

  const openCreate = () => {
    setEditing(null);
    form.resetFields();
    form.setFieldsValue({ log_date: dayjs(), area_id: areaId, worker_count: 1, work_hours: 8, category: '采收', paid_status: 'unpaid' });
    updateAmount();
    setModalOpen(true);
  };

  const openEdit = (r: LaborRecord) => {
    setEditing(r);
    form.setFieldsValue({
      log_date: dayjs(r.log_date), area_id: r.area_id || undefined, category: r.category,
      worker: r.worker, worker_count: r.worker_count, work_hours: r.work_hours, rate: r.rate,
      paid_status: r.paid_status, operator: r.operator, notes: r.notes,
    });
    updateAmount();
    setModalOpen(true);
  };

  const handleSave = async () => {
    const v = await form.validateFields();
    const payload = {
      ...v,
      log_date: v.log_date.format('YYYY-MM-DD'),
    };
    delete payload.amount_preview;
    try {
      if (editing) {
        await laborApi.updateRecord(editing.id, payload);
        message.success('已更新');
      } else {
        await laborApi.createRecord(payload);
        message.success('已记录用工');
      }
      setModalOpen(false);
      load();
    } catch {
      message.error('保存失败');
    }
  };

  const by_date = summary?.by_date || [];
  const maxDayCost = Math.max(...by_date.map(d => d.amount), 1);

  const columns = [
    { title: '日期', dataIndex: 'log_date', key: 'date', width: 105 },
    { title: '区域', dataIndex: 'area_id', key: 'area', width: 100, render: (_: string, r: LaborRecord) => zones.find(z => z.id === r.area_id)?.name || '-' },
    { title: '类别', dataIndex: 'category', key: 'cat', width: 80, render: (v: string) => <Tag>{v}</Tag> },
    { title: '用工', dataIndex: 'worker', key: 'worker', width: 90, render: (v: string) => v || <Text type="secondary">全员</Text> },
    { title: '人数', dataIndex: 'worker_count', key: 'wc', width: 60 },
    { title: '工时/人', dataIndex: 'work_hours', key: 'wh', width: 75, render: (v: number) => `${v}h` },
    { title: '单价', dataIndex: 'rate', key: 'rate', width: 70, render: (v: number) => `¥${v}/h` },
    { title: '金额', dataIndex: 'amount', key: 'amount', width: 90, render: (v: number) => <b>¥{v.toFixed(2)}</b> },
    { title: '结算', dataIndex: 'paid_status', key: 'paid', width: 70, render: (v: string) => v === 'paid' ? <Tag color="green">已结</Tag> : <Tag color="orange">未结</Tag> },
    { title: '备注', dataIndex: 'notes', key: 'notes', ellipsis: true },
    {
      title: '操作', key: 'actions', width: 110,
      render: (_: unknown, r: LaborRecord) => (
        <Space size={4}>
          <Button size="small" icon={<EditOutlined />} onClick={() => openEdit(r)} />
          <Popconfirm title="删除该用工记录？" onConfirm={async () => {
            await laborApi.deleteRecord(r.id);
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
      <Title level={4} style={{ margin: 0, marginBottom: 16 }}>👷 用工成本</Title>

      <Row gutter={[16, 12]} style={{ marginBottom: 16 }}>
        <Col xs={12} sm={6}><Card><Statistic title="用工总成本" value={summary?.total_amount || 0} precision={2} prefix="¥" valueStyle={{ color: '#cf1322' }} /></Card></Col>
        <Col xs={12} sm={6}><Card><Statistic title="用工天数" value={summary?.total_workdays || 0} suffix="天" /></Card></Col>
        <Col xs={12} sm={6}><Card><Statistic title="总人天" value={summary?.total_worker_days || 0} suffix="人·天" /></Card></Col>
        <Col xs={12} sm={6}><Card><Statistic title="记录数" value={summary?.record_count || 0} suffix="条" /></Card></Col>
      </Row>

      <Row gutter={[16, 12]} style={{ marginBottom: 16 }}>
        <Col xs={24} md={14}>
          <Card title={<Space><CalendarOutlined />每日用工成本</Space>} size="small" styles={{ body: { maxHeight: 220, overflowY: 'auto' } }}>
            {by_date.length === 0 ? <Empty description="暂无数据" /> : (
              <div>
                {by_date.map(d => (
                  <div key={d.log_date} style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                    <Text style={{ width: 90, fontSize: 12 }}>{d.log_date}</Text>
                    <div style={{ flex: 1, background: '#f0f0f0', borderRadius: 3, height: 18 }}>
                      <div style={{
                        width: `${(d.amount / maxDayCost) * 100}%`, background: '#fa8c16',
                        height: '100%', borderRadius: 3, minWidth: d.amount > 0 ? 4 : 0,
                      }} />
                    </div>
                    <Text style={{ width: 90, fontSize: 12, textAlign: 'right' }}>¥{d.amount.toFixed(2)}</Text>
                    <Text type="secondary" style={{ width: 55, fontSize: 12, textAlign: 'right' }}>{d.worker_count}人</Text>
                  </div>
                ))}
              </div>
            )}
          </Card>
        </Col>
        <Col xs={24} md={10}>
          <Card title={<Space><UserOutlined />类别成本分布</Space>} size="small" styles={{ body: { maxHeight: 220, overflowY: 'auto' } }}>
            {(summary?.by_category || []).length === 0 ? <Empty description="暂无数据" /> : (
              (summary?.by_category || []).map(c => (
                <div key={c.category} style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 6, fontSize: 13 }}>
                  <span>{c.category} <Text type="secondary">({c.count}次)</Text></span>
                  <span>¥{c.amount.toFixed(2)}</span>
                </div>
              ))
            )}
          </Card>
        </Col>
      </Row>

      <Alert
        type="info"
        showIcon
        style={{ marginBottom: 12 }}
        message={`金额 = 人数 × 工时/人 × 单价（¥${(() => { const r = records[0]; return r ? r.rate : 0; })()}/h/人）`}
      />

      <Card
        title={
          <Space><DollarOutlined />用工记录</Space>
        }
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
            <Button type="primary" icon={<PlusOutlined />} onClick={openCreate}>记录用工</Button>
          </Space>
        }
      >
        <Table rowKey="id" columns={columns} dataSource={records} loading={loading} pagination={{ pageSize: 10, total, onChange: load }} size="middle" scroll={{ x: 'max-content' }} />
      </Card>

      <Modal
        title={editing ? '编辑用工记录' : '记录用工'}
        open={modalOpen}
        onOk={handleSave}
        onCancel={() => setModalOpen(false)}
        width={520}
      >
        <Form form={form} layout="vertical">
          <Row gutter={12}>
            <Col xs={24} sm={12}><Form.Item name="log_date" label="用工日期" rules={[{ required: true }]}><DatePicker style={{ width: '100%' }} /></Form.Item></Col>
            <Col xs={24} sm={12}>
              <Form.Item name="area_id" label="区域"><Select allowClear options={zones.map(z => ({ value: z.id, label: z.name }))} /></Form.Item>
            </Col>
          </Row>
          <Row gutter={12}>
            <Col xs={24} sm={12}><Form.Item name="category" label="作业类别" rules={[{ required: true }]}><Select options={LABOR_CATEGORIES.map(c => ({ value: c, label: c }))} /></Form.Item></Col>
            <Col xs={24} sm={12}><Form.Item name="worker" label="用工（留空=整组）"><Input placeholder="如：张三 / 王五大姐等" /></Form.Item></Col>
          </Row>
          <Row gutter={12}>
            <Col xs={8}><Form.Item name="worker_count" label="人数" rules={[{ required: true }]}><InputNumber style={{ width: '100%' }} min={1} onChange={updateAmount} /></Form.Item></Col>
            <Col xs={8}><Form.Item name="work_hours" label="工时/人(h)" rules={[{ required: true }]}><InputNumber style={{ width: '100%' }} min={0} onChange={updateAmount} /></Form.Item></Col>
            <Col xs={8}><Form.Item name="rate" label="单价(元/h)" rules={[{ required: true }]}><InputNumber style={{ width: '100%' }} min={0} onChange={updateAmount} /></Form.Item></Col>
          </Row>
          <Form.Item label="预计金额">
            <Form.Item name="amount_preview" noStyle><InputNumber disabled style={{ width: '100%' }} /></Form.Item>
          </Form.Item>
          <Form.Item name="paid_status" label="结算状态">
            <Select options={[{ value: 'unpaid', label: '未结' }, { value: 'paid', label: '已结' }]} />
          </Form.Item>
          <Row gutter={12}>
            <Col xs={24} sm={12}><Form.Item name="operator" label="记录人"><Input /></Form.Item></Col>
          </Row>
          <Form.Item name="notes" label="备注"><Input.TextArea rows={2} /></Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default Labor;