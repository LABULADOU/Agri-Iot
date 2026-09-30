import React, { useEffect, useState } from 'react';
import {
  Typography, Card, Row, Col, Button, Form, Input, InputNumber, Select, Space, Tag,
  Table, Tabs, Popconfirm, Modal, message, Alert, Descriptions, Spin,
} from 'antd';
import {
  ExperimentOutlined, BugOutlined, ThunderboltOutlined, CheckCircleOutlined,
  PlusOutlined, DeleteOutlined,
} from '@ant-design/icons';
import { mixingApi, zoneApi } from '../../services/api';
import type { MixingPlan, MixingRecipe, MixingPreset } from '../../types';

const { Title, Text } = Typography;

const STAGE_OPTIONS = [
  { value: 'seedling', label: '苗期 (0-15天)' },
  { value: 'vegetative', label: '营养生长期 (15-40天)' },
  { value: 'flowering', label: '开花期 (40-60天)' },
  { value: 'fruiting', label: '结果期 (60天+)' },
];

const StageTag: React.FC<{ plan?: MixingPlan }> = ({ plan }) => {
  if (!plan?.stage) return null;
  return <Tag color="purple">{plan.stage}</Tag>;
};

const PlanView: React.FC<{ plan: MixingPlan; onApply: () => void; applying: boolean; applied: boolean }> = ({ plan, onApply, applying, applied }) => {
  interface PlanItem { name?: string; unit?: string; n?: number; p?: number; k?: number; amount?: number }
  const items = (plan.plan?.items || []) as PlanItem[];
  return (
    <Card
      size="small"
      style={{ marginTop: 12, borderLeft: '3px solid #1677ff' }}
      title={
        <Space>
          {plan.type === 'fertilizer' ? <ExperimentOutlined /> : <BugOutlined />}
          {plan.type === 'fertilizer' ? '配肥方案' : '配药方案'}
          <StageTag plan={plan} />
          {plan.source === 'preset' && <Tag color="gold">预设</Tag>}
          {plan.source === 'knowledge' && <Tag color="blue">知识库</Tag>}
          {plan.source === 'fallback' && <Tag color="red">需人工</Tag>}
        </Space>
      }
      extra={
        !applied && plan.source !== 'fallback' ? (
          <Button type="primary" size="small" icon={<ThunderboltOutlined />} loading={applying} onClick={onApply}>
            应用到日志并扣库存
          </Button>
        ) : applied ? <Tag color="green" icon={<CheckCircleOutlined />}>已应用</Tag> : null
      }
    >
      <Descriptions size="small" column={{ xs: 1, sm: 2 }} style={{ marginBottom: 8 }}>
        {plan.plan?.dilution && <Descriptions.Item label="稀释比例">{plan.plan.dilution}</Descriptions.Item>}
        {plan.plan?.ec_target !== undefined && <Descriptions.Item label="EC 目标">{plan.plan.ec_target} mS/cm</Descriptions.Item>}
        {plan.plan?.water_volume !== undefined && <Descriptions.Item label="用水量">{plan.plan.water_volume}{typeof plan.plan.water_volume === 'number' ? ' L' : ''}</Descriptions.Item>}
        {plan.plan?.safety_interval_days !== undefined && <Descriptions.Item label="安全间隔期">{plan.plan.safety_interval_days} 天</Descriptions.Item>}
        {plan.growth_days !== undefined && <Descriptions.Item label="生长天数">{plan.growth_days} 天</Descriptions.Item>}
        {plan.target_pest && <Descriptions.Item label="防治对象">{plan.target_pest}</Descriptions.Item>}
      </Descriptions>

      <Table<PlanItem>
        rowKey={(r, i) => `${r?.name || 'item'}-${i}`}
        size="small"
        dataSource={items}
        pagination={false}
        scroll={{ x: 'max-content' }}
        columns={[
          { title: '投入品', dataIndex: 'name', render: (v: string) => <Text strong>{v || '-'}</Text> },
          { title: 'N', dataIndex: 'n', width: 60, render: (v?: number) => v !== undefined ? `${v}%` : '-' },
          { title: 'P', dataIndex: 'p', width: 60, render: (v?: number) => v !== undefined ? `${v}%` : '-' },
          { title: 'K', dataIndex: 'k', width: 60, render: (v?: number) => v !== undefined ? `${v}%` : '-' },
          { title: '用量', dataIndex: 'amount', width: 110, render: (v?: number, r?: PlanItem) => v ? `${v} ${r?.unit || ''}` : '-' },
        ]}
      />

      {plan.adjustments?.length > 0 && (
        <Alert style={{ marginTop: 8 }} type="warning" showIcon message="修正建议" description={
          <div>{plan.adjustments.map((a, i) => <div key={i}>· {a}</div>)}</div>
        } />
      )}
      {plan.reasoning && <Text type="secondary" style={{ display: 'block', marginTop: 8, fontSize: 12 }}>推理：{plan.reasoning}</Text>}
      {plan.treatment && <Alert style={{ marginTop: 8 }} type="info" showIcon message="防治建议" description={plan.treatment} />}
    </Card>
  );
};

const Mixing: React.FC = () => {
  const [zones, setZones] = useState<Array<{ id: string; name: string }>>([]);
  const [activeTab, setActiveTab] = useState('fertilizer');

  // recommend forms
  const [fertiForm] = Form.useForm();
  const [pestForm] = Form.useForm();
  const [fertiPlan, setFertiPlan] = useState<MixingPlan | null>(null);
  const [pestPlan, setPestPlan] = useState<MixingPlan | null>(null);
  const [loadingF, setLoadingF] = useState(false);
  const [loadingP, setLoadingP] = useState(false);
  const [applying, setApplying] = useState(false);

  const [recipes, setRecipes] = useState<MixingRecipe[]>([]);
  const [presets, setPresets] = useState<MixingPreset[]>([]);
  const [presetModal, setPresetModal] = useState(false);
  const [presetForm] = Form.useForm();

  useEffect(() => {
    zoneApi.list().then(setZones).catch(() => {});
    mixingApi.listRecipes()
      .then(data => setRecipes(data.recipes || []))
      .catch(() => {});
    mixingApi.listPresets()
      .then(data => setPresets(data.presets || []))
      .catch(() => {});
  }, []);

  const loadRecipes = async () => {
    try {
      const data = await mixingApi.listRecipes();
      setRecipes(data.recipes || []);
    } catch { /* ignore */ }
  };

  const loadPresets = async (type?: string) => {
    try {
      const data = await mixingApi.listPresets(type);
      setPresets(data.presets || []);
    } catch { /* ignore */ }
  };

  const genFertilizer = async () => {
    const v = await fertiForm.validateFields();
    setLoadingF(true);
    setFertiPlan(null);
    try {
      const plan = await mixingApi.recommendFertilizer({
        area_id: v.area_id,
        crop_batch_id: v.crop_batch_id || undefined,
        growth_days: v.growth_days || undefined,
      });
      setFertiPlan(plan);
      await loadRecipes();
    } catch {
      message.error('生成配肥方案失败');
    } finally {
      setLoadingF(false);
    }
  };

  const genPesticide = async () => {
    const v = await pestForm.validateFields();
    setLoadingP(true);
    setPestPlan(null);
    try {
      const plan = await mixingApi.recommendPesticide({
        area_id: v.area_id,
        crop_batch_id: v.crop_batch_id || undefined,
        target_pest: v.target_pest,
      });
      setPestPlan(plan);
      await loadRecipes();
    } catch {
      message.error('生成配药方案失败');
    } finally {
      setLoadingP(false);
    }
  };

  const applyRecipe = async (recipeId: string) => {
    setApplying(true);
    try {
      const res = await mixingApi.applyRecipe(recipeId);
      message.success('已生成农事日志并扣减库存');
      if (res.warnings?.length) {
        res.warnings.forEach(w => message.warning(w));
      }
      setFertiPlan(null);
      setPestPlan(null);
      await loadRecipes();
    } catch {
      message.error('应用失败');
    } finally {
      setApplying(false);
    }
  };

  const recipeColumns = [
    { title: '时间', dataIndex: 'created_at', width: 140, render: (v: number) => new Date(v * 1000).toLocaleString() },
    {
      title: '类型', dataIndex: 'mix_type', width: 80,
      render: (v: string) => v === 'fertilizer' ? <Tag color="blue">配肥</Tag> : <Tag color="red">配药</Tag>,
    },
    { title: '区域', dataIndex: 'area_name', width: 100, render: (v: string) => v || '-' },
    {
      title: '方案', key: 'plan', render: (_: unknown, r: MixingRecipe) => {
        const items = r.result?.plan?.items || [];
        return items.map((i, idx) => <Tag key={idx} style={{ marginBottom: 2 }}>{i.name}</Tag>);
      },
    },
    {
      title: '状态', dataIndex: 'status', width: 100,
      render: (v: string) => v === 'applied' ? <Tag color="green">已应用</Tag> : <Tag color="orange">未应用</Tag>,
    },
    {
      title: '操作', key: 'actions', width: 120,
      render: (_: unknown, r: MixingRecipe) => r.status === 'generated' ? (
        <Button type="primary" size="small" onClick={() => applyRecipe(r.id)}>应用</Button>
      ) : (
        <Text type="secondary" style={{ fontSize: 12 }}>{r.farm_op_id ? '已生成日志' : ''}</Text>
      ),
    },
  ];

  const presetColumns = [
    { title: '名称', dataIndex: 'name' },
    {
      title: '类型', dataIndex: 'mix_type', width: 80,
      render: (v: string) => v === 'fertilizer' ? <Tag color="blue">配肥</Tag> : <Tag color="red">配药</Tag>,
    },
    { title: '阶段', dataIndex: 'stage_key', width: 110, render: (v: string) => STAGE_OPTIONS.find(s => s.value === v)?.label || '-' },
    { title: '稀释', dataIndex: 'dilution', width: 90, render: (v: string) => v || '-' },
    { title: '安全间隔', dataIndex: 'safety_interval_days', width: 90, render: (v: number) => v ? `${v}天` : '-' },
    { title: '备注', dataIndex: 'note', ellipsis: true },
    {
      title: '操作', key: 'actions', width: 90,
      render: (_: unknown, r: MixingPreset) => (
        <Popconfirm title="删除该预设？" onConfirm={async () => {
          await mixingApi.deletePreset(r.id);
          message.success('已删除');
          loadPresets();
        }} okText="删除" cancelText="取消">
          <Button size="small" danger icon={<DeleteOutlined />} />
        </Popconfirm>
      ),
    },
  ];

  const savePreset = async () => {
    const v = await presetForm.validateFields();
    try {
      await mixingApi.createPreset({
        mix_type: v.mix_type,
        name: v.name,
        stage_key: v.stage_key,
        dilution: v.dilution,
        safety_interval_days: v.safety_interval_days || 7,
        note: v.note,
        items: [{ name: v.name }],
      });
      message.success('已保存');
      setPresetModal(false);
      loadPresets(v.mix_type);
    } catch {
      message.error('保存失败');
    }
  };

  return (
    <div style={{ padding: 16 }}>
      <Title level={4} style={{ margin: 0, marginBottom: 16 }}>🧪 配肥配药</Title>
      <Alert
        style={{ marginBottom: 16 }}
        type="info"
        showIcon
        message="AI 辅助生成方案"
        description="根据作物生长阶段（种植天数）、土壤 EC/温度传感器数据、病虫害知识库自动生成配方。应用后自动创建农事日志（计划状态）并按名称匹配库存自动扣减。"
      />

      <Tabs
        activeKey={activeTab}
        onChange={setActiveTab}
        items={[
          {
            key: 'fertilizer',
            label: <span><ExperimentOutlined /> 配肥</span>,
            children: (
              <Row gutter={[16, 12]}>
                <Col xs={24} md={10}>
                  <Card title="环境与作物信息" size="small">
                    <Form form={fertiForm} layout="vertical" initialValues={{ growth_days: 20 }}>
                      <Form.Item name="area_id" label="区域" rules={[{ required: true }]}>
                        <Select options={zones.map(z => ({ value: z.id, label: z.name }))} placeholder="选择区域" />
                      </Form.Item>
                      <Form.Item name="growth_days" label="生长天数（自动推算种植后第N天）">
                        <InputNumber style={{ width: '100%' }} min={0} max={365} />
                      </Form.Item>
                      <Button type="primary" icon={<ThunderboltOutlined />} loading={loadingF} onClick={genFertilizer} block>
                        生成配肥方案
                      </Button>
                      <Text type="secondary" style={{ display: 'block', marginTop: 8, fontSize: 12 }}>
                        引擎将读取该区域最新土壤 EC/温度，结合作物 EC 目标与阶段基准公式（20-20-20 → 30-10-20 → 10-30-20 → 15-10-35）计算用量
                      </Text>
                    </Form>
                  </Card>
                </Col>
                <Col xs={24} md={14}>
                  {loadingF ? <div style={{ textAlign: 'center', padding: 60 }}><Spin tip="生成中" /></div> : (
                    fertiPlan ? (
                      <PlanView
                        plan={fertiPlan}
                        applying={applying}
                        applied={false}
                        onApply={() => {
                          const id = recipes.find(r => r.mix_type === 'fertilizer' && r.status === 'generated')?.id;
                          if (id) applyRecipe(id);
                        }}
                      />
                    ) : (
                      <Card size="small"><Text type="secondary">填写左侧表单生成配肥方案</Text></Card>
                    )
                  )}
                </Col>
              </Row>
            ),
          },
          {
            key: 'pesticide',
            label: <span><BugOutlined /> 配药</span>,
            children: (
              <Row gutter={[16, 12]}>
                <Col xs={24} md={10}>
                  <Card title="病虫害信息" size="small">
                    <Form form={pestForm} layout="vertical" initialValues={{ target_pest: '白粉虱' }}>
                      <Form.Item name="area_id" label="区域" rules={[{ required: true }]}>
                        <Select options={zones.map(z => ({ value: z.id, label: z.name }))} placeholder="选择区域" />
                      </Form.Item>
                      <Form.Item name="target_pest" label="病虫害名称" rules={[{ required: true }]}>
                        <Input placeholder="如：白粉虱、霜霉病、红蜘蛛" />
                      </Form.Item>
                      <Button type="primary" icon={<ThunderboltOutlined />} loading={loadingP} onClick={genPesticide} block>
                        生成配药方案
                      </Button>
                      <Text type="secondary" style={{ display: 'block', marginTop: 8, fontSize: 12 }}>
                        引擎将检索 pest_knowledge 知识库与预设方案；未命中时提示补充知识库
                      </Text>
                    </Form>
                  </Card>
                </Col>
                <Col xs={24} md={14}>
                  {loadingP ? <div style={{ textAlign: 'center', padding: 60 }}><Spin tip="生成中" /></div> : (
                    pestPlan ? (
                      <PlanView
                        plan={pestPlan}
                        applying={applying}
                        applied={false}
                        onApply={() => {
                          const id = recipes.find(r => r.mix_type === 'pesticide' && r.status === 'generated')?.id;
                          if (id) applyRecipe(id);
                        }}
                      />
                    ) : (
                      <Card size="small"><Text type="secondary">填写左侧表单生成配药方案</Text></Card>
                    )
                  )}
                </Col>
              </Row>
            ),
          },
          {
            key: 'recipes',
            label: '方案历史',
            children: <Table rowKey="id" size="middle" columns={recipeColumns} dataSource={recipes} pagination={{ pageSize: 10 }} scroll={{ x: 'max-content' }} />,
          },
          {
            key: 'presets',
            label: '预设管理',
            children: (
              <div>
                <div style={{ marginBottom: 12, display: 'flex', justifyContent: 'space-between' }}>
                  <Text type="secondary">预设方案优先于自动公式，可覆盖为你确认过的配方</Text>
                  <Button type="primary" size="small" icon={<PlusOutlined />} onClick={() => {
                    presetForm.resetFields();
                    presetForm.setFieldsValue({ mix_type: 'fertilizer' });
                    setPresetModal(true);
                  }}>新增预设</Button>
                </div>
                <Table rowKey="id" size="middle" columns={presetColumns} dataSource={presets} pagination={false} scroll={{ x: 'max-content' }} />
              </div>
            ),
          },
        ]}
      />

      <Modal
        title="新增预设方案"
        open={presetModal}
        onOk={savePreset}
        onCancel={() => setPresetModal(false)}
      >
        <Form form={presetForm} layout="vertical">
          <Form.Item name="mix_type" label="类型" rules={[{ required: true }]}>
            <Select options={[{ value: 'fertilizer', label: '配肥' }, { value: 'pesticide', label: '配药' }]} />
          </Form.Item>
          <Form.Item name="name" label="名称" rules={[{ required: true }]}>
            <Input placeholder="如：白粉虱预案 / 结果期高钾配方" />
          </Form.Item>
          <Form.Item name="stage_key" label="适用阶段（配肥）">
            <Select allowClear options={STAGE_OPTIONS} />
          </Form.Item>
          <Row gutter={12}>
            <Col xs={24} sm={12}><Form.Item name="dilution" label="稀释比例"><Input placeholder="如：2000倍" /></Form.Item></Col>
            <Col xs={24} sm={12}><Form.Item name="safety_interval_days" label="安全间隔期（天，配药）"><InputNumber style={{ width: '100%' }} min={0} /></Form.Item></Col>
          </Row>
          <Form.Item name="note" label="备注"><Input.TextArea rows={2} /></Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default Mixing;