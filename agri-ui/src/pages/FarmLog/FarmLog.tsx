import React, { useEffect, useState, useCallback } from 'react';
import {
  Typography, Select, DatePicker, Button, Drawer, Form, Input, Space, Spin, Empty,
  Modal, message, Tag, Tooltip, Popconfirm,
} from 'antd';
import {
  PlusOutlined, DeleteOutlined, EditOutlined, FileTextOutlined,
  BugOutlined, EnvironmentOutlined,
} from '@ant-design/icons';
import dayjs from 'dayjs';
import { farmApi, zoneApi } from '../../services/api';
import type { FarmOperation, Zone, FarmOpCategory, FarmOpTemplate } from '../../types';
import PesticideForm from './PesticideForm';
import FertilizerForm from './FertilizerForm';
import styles from './FarmLog.module.css';

const { Title, Text } = Typography;
const { TextArea } = Input;

const CATEGORY_OPTIONS: { value: FarmOpCategory; label: string }[] = [
  { value: '打药', label: '打药' },
  { value: '施肥', label: '施肥' },
  { value: '灌溉', label: '灌溉' },
  { value: '修剪', label: '修剪' },
  { value: '采收', label: '采收' },
  { value: '设备维护', label: '设备维护' },
  { value: '定植', label: '定植' },
  { value: '育苗', label: '育苗' },
  { value: '巡棚', label: '巡棚' },
  { value: '其他', label: '其他' },
];

const categoryClass: Record<string, string> = {
  '打药': styles.categoryPesticide,
  '施肥': styles.categoryFertilizer,
  '灌溉': styles.categoryIrrigation,
  '修剪': styles.categoryPruning,
  '采收': styles.categoryHarvest,
};

function getCategoryClass(cat: string): string {
  return categoryClass[cat] || styles.categoryOther;
}

interface OpFormData {
  area_id: string;
  log_date: string;
  log_time: string;
  category: FarmOpCategory;
  content: string;
  operator: string;
  weather: string;
  crop_status: string;
  notes: string;
  details: Record<string, unknown>;
}

const emptyForm: OpFormData = {
  area_id: '',
  log_date: dayjs().format('YYYY-MM-DD'),
  log_time: dayjs().format('HH:mm'),
  category: '巡棚',
  content: '',
  operator: '',
  weather: '',
  crop_status: '',
  notes: '',
  details: {},
};

const FarmLog: React.FC = () => {
  const [operations, setOperations] = useState<FarmOperation[]>([]);
  const [zones, setZones] = useState<Zone[]>([]);
  const [templates, setTemplates] = useState<FarmOpTemplate[]>([]);
  const [loading, setLoading] = useState(true);
  const [areaId, setAreaId] = useState<string>('');
  const [dateRange, setDateRange] = useState<[dayjs.Dayjs | null, dayjs.Dayjs | null]>([dayjs().subtract(7, 'day'), dayjs()]);
  const [categoryFilter, setCategoryFilter] = useState<string>('');

  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [formData, setFormData] = useState<OpFormData>({ ...emptyForm });
  const [saving, setSaving] = useState(false);

  const [detailDrawer, setDetailDrawer] = useState<FarmOperation | null>(null);

  const loadZones = useCallback(async () => {
    try {
      const data = await zoneApi.list();
      setZones(data);
      if (!areaId && data.length > 0) setAreaId(data[0].id);
    } catch { /* ignore */ }
  }, []);

  const loadOps = useCallback(async () => {
    try {
      const params: Record<string, string> = {};
      if (areaId) params.area_id = areaId;
      if (dateRange[0]) params.date_from = dateRange[0].format('YYYY-MM-DD');
      if (dateRange[1]) params.date_to = dateRange[1].format('YYYY-MM-DD');
      if (categoryFilter) params.category = categoryFilter;
      const data = await farmApi.listOps(params);
      setOperations(data.operations);
    } catch { message.error('加载农事日志失败'); }
  }, [areaId, dateRange, categoryFilter]);

  const loadTemplates = useCallback(async () => {
    try {
      const data = await farmApi.listTemplates();
      setTemplates(data);
    } catch { /* ignore */ }
  }, []);

  useEffect(() => {
    loadZones();
    loadTemplates();
  }, []);

  useEffect(() => {
    if (areaId) {
      setLoading(true);
      loadOps().finally(() => setLoading(false));
    } else {
      setLoading(false);
    }
  }, [areaId, dateRange, categoryFilter]);

  const openCreate = (category?: FarmOpCategory) => {
    setEditingId(null);
    setFormData({
      ...emptyForm,
      area_id: areaId,
      log_date: dayjs().format('YYYY-MM-DD'),
      log_time: dayjs().format('HH:mm'),
      category: category || '巡棚',
    });
    setDrawerOpen(true);
  };

  const openEdit = (op: FarmOperation) => {
    setEditingId(op.id);
    setFormData({
      area_id: op.area_id,
      log_date: op.log_date,
      log_time: op.log_time,
      category: op.category,
      content: op.content,
      operator: op.operator,
      weather: op.weather,
      crop_status: op.crop_status,
      notes: op.notes,
      details: op.details as Record<string, unknown>,
    });
    setDrawerOpen(true);
  };

  const applyTemplate = (template: FarmOpTemplate) => {
    setFormData(prev => ({
      ...prev,
      category: template.category,
      details: template.details as Record<string, unknown>,
      content: (template.details as any)?.target_pest
        ? `防治${(template.details as any).target_pest}`
        : (template.details as any)?.items?.[0]?.name
          ? `施用${(template.details as any).items.map((i: any) => i.name).join('、')}`
          : template.name,
    }));
  };

  const handleSave = async () => {
    if (!formData.area_id || !formData.content) {
      message.warning('请填写区域和操作内容');
      return;
    }
    setSaving(true);
    try {
      const payload = {
        ...formData,
        details: formData.category === '打药' || formData.category === '施肥' ? formData.details : undefined,
      };
      if (editingId) {
        await farmApi.updateOp(editingId, payload);
        message.success('已更新');
      } else {
        await farmApi.createOp(payload as any);
        message.success('已创建');
      }
      setDrawerOpen(false);
      loadOps();
    } catch {
      message.error('保存失败');
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await farmApi.deleteOp(id);
      message.success('已删除');
      loadOps();
    } catch {
      message.error('删除失败');
    }
  };

  const groupedByDate = Array.isArray(operations) ? operations.reduce<Record<string, FarmOperation[]>>((acc, op) => {
    if (!acc[op.log_date]) acc[op.log_date] = [];
    acc[op.log_date].push(op);
    return acc;
  }, {}) : {};

  const sortedDates = Object.keys(groupedByDate).sort((a, b) => b.localeCompare(a));

  const templatesByCategory = Array.isArray(templates) ? templates.reduce<Record<string, FarmOpTemplate[]>>((acc, t) => {
    if (!acc[t.category]) acc[t.category] = [];
    acc[t.category].push(t);
    return acc;
  }, {}) : {};

  return (
    <div className={styles.container}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <Title level={4} style={{ margin: 0 }}>📋 农事日志</Title>
      </div>

      <div className={styles.toolbar}>
        <div className={styles.toolbarLeft}>
          <Select
            value={areaId}
            onChange={setAreaId}
            placeholder="选择区域"
            style={{ width: 180 }}
            options={zones.map(z => ({ value: z.id, label: z.name }))}
          />
          <DatePicker.RangePicker
            value={dateRange}
            onChange={vals => setDateRange(vals || [null, null])}
            allowClear={false}
            size="middle"
          />
          <Select
            value={categoryFilter}
            onChange={setCategoryFilter}
            placeholder="全部类别"
            allowClear
            style={{ width: 130 }}
            options={CATEGORY_OPTIONS.map(c => ({ value: c.value, label: c.label }))}
            onClear={() => setCategoryFilter('')}
          />
        </div>
        <div className={styles.toolbarRight}>
          <Button type="primary" icon={<PlusOutlined />} onClick={() => openCreate()}>
            新建操作
          </Button>
        </div>
      </div>

      {loading ? (
        <div style={{ textAlign: 'center', padding: 60 }}><Spin /></div>
      ) : operations.length === 0 ? (
        <div className={styles.emptyState}>
          <FileTextOutlined className={styles.emptyIcon} />
          <Text type="secondary">暂无农事记录</Text>
          <Text type="secondary" style={{ fontSize: 13, marginTop: 4 }}>
            选择区域和时间范围后，点击"新建操作"添加记录
          </Text>
        </div>
      ) : (
        <div className={styles.operationList}>
          {sortedDates.map(date => (
            <div key={date} className={styles.dateGroup}>
              <div className={styles.dateHeader}>
                <span>{date} <Text type="secondary" style={{ fontSize: 12, fontWeight: 400 }}>（{groupedByDate[date].length} 条）</Text></span>
              </div>
              {groupedByDate[date].map(op => (
                <div key={op.id} className={styles.opCard} onClick={() => setDetailDrawer(op)}>
                  <div className={styles.opTime}>{op.log_time || '--:--'}</div>
                  <div>
                    <span className={`${styles.opCategory} ${getCategoryClass(op.category)}`}>{op.category}</span>
                  </div>
                  <div className={styles.opContent}>
                    <div className={styles.contentText}>{op.content}</div>
                  </div>
                  <div className={styles.opMeta}>
                    <span>{op.operator || '--'}</span>
                    <Tooltip title="编辑">
                      <Button type="text" size="small" icon={<EditOutlined />}
                        onClick={e => { e.stopPropagation(); openEdit(op); }} />
                    </Tooltip>
                    <Popconfirm title="确定删除？" onConfirm={e => { e?.stopPropagation(); handleDelete(op.id); }}
                      onCancel={e => e?.stopPropagation()} okText="删除" cancelText="取消">
                      <Button type="text" size="small" danger icon={<DeleteOutlined />}
                        onClick={e => e.stopPropagation()} />
                    </Popconfirm>
                  </div>
                </div>
              ))}
            </div>
          ))}
        </div>
      )}

      {/* Create/Edit Drawer */}
      <Drawer
        title={editingId ? '编辑操作' : '新建操作'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={640}
        extra={
          <Button type="primary" onClick={handleSave} loading={saving}>
            {editingId ? '更新' : '保存'}
          </Button>
        }
      >
        <Space direction="vertical" style={{ width: '100%' }} size={12}>
          <Space style={{ width: '100%' }}>
            <Form.Item label="日期" style={{ marginBottom: 0, flex: 1 }}>
              <DatePicker value={dayjs(formData.log_date)} onChange={d => setFormData(prev => ({ ...prev, log_date: d?.format('YYYY-MM-DD') || '' }))} style={{ width: '100%' }} />
            </Form.Item>
            <Form.Item label="时间" style={{ marginBottom: 0, width: 100 }}>
              <Input value={formData.log_time} onChange={e => setFormData(prev => ({ ...prev, log_time: e.target.value }))} placeholder="08:30" />
            </Form.Item>
          </Space>

          <Space style={{ width: '100%' }}>
            <Form.Item label="类别" style={{ marginBottom: 0, width: 140 }}>
              <Select value={formData.category} onChange={v => setFormData(prev => ({ ...prev, category: v as FarmOpCategory, details: {} }))} options={CATEGORY_OPTIONS} style={{ width: '100%' }} />
            </Form.Item>
            <Form.Item label="操作人" style={{ marginBottom: 0, flex: 1 }}>
              <Input value={formData.operator} onChange={e => setFormData(prev => ({ ...prev, operator: e.target.value }))} placeholder="姓名" />
            </Form.Item>
          </Space>

          {/* Template quick insert */}
          {templatesByCategory[formData.category]?.length > 0 && (
            <div>
              <Text type="secondary" style={{ fontSize: 12 }}>快速模板：</Text>
              <Space wrap style={{ marginTop: 4 }}>
                {templatesByCategory[formData.category].map(t => (
                  <Tag key={t.id} color="blue" style={{ cursor: 'pointer' }} onClick={() => applyTemplate(t)}>
                    {t.name}
                  </Tag>
                ))}
              </Space>
            </div>
          )}

          <Form.Item label="操作内容" style={{ marginBottom: 0 }}>
            <TextArea value={formData.content} onChange={e => setFormData(prev => ({ ...prev, content: e.target.value }))} rows={2} placeholder="描述操作内容" />
          </Form.Item>

          <Form.Item label="天气" style={{ marginBottom: 0 }}>
            <Input value={formData.weather} onChange={e => setFormData(prev => ({ ...prev, weather: e.target.value }))} placeholder="如：晴 25°C" />
          </Form.Item>

          <Form.Item label="作物状态" style={{ marginBottom: 0 }}>
            <TextArea value={formData.crop_status} onChange={e => setFormData(prev => ({ ...prev, crop_status: e.target.value }))} rows={2} placeholder="如：植株健壮，花蕾饱满" />
          </Form.Item>

          {/* Category-specific detail forms */}
          {formData.category === '打药' && (
            <div style={{ border: '1px solid #f0f0f0', borderRadius: 8, padding: 12 }}>
              <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8, color: '#cf1322' }}>
                <BugOutlined /> 打药记录详情
              </div>
              <PesticideForm
                value={formData.details as any}
                onChange={val => setFormData(prev => ({ ...prev, details: val }))}
              />
            </div>
          )}

          {formData.category === '施肥' && (
            <div style={{ border: '1px solid #f0f0f0', borderRadius: 8, padding: 12 }}>
              <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8, color: '#389e0d' }}>
                <EnvironmentOutlined /> 施肥记录详情
              </div>
              <FertilizerForm
                value={formData.details as any}
                onChange={val => setFormData(prev => ({ ...prev, details: val }))}
              />
            </div>
          )}

          <Form.Item label="备注" style={{ marginBottom: 0 }}>
            <TextArea value={formData.notes} onChange={e => setFormData(prev => ({ ...prev, notes: e.target.value }))} rows={2} />
          </Form.Item>
        </Space>
      </Drawer>

      {/* Detail Drawer */}
      <Drawer
        title="操作详情"
        open={!!detailDrawer}
        onClose={() => setDetailDrawer(null)}
        width={480}
      >
        {detailDrawer && (
          <Space direction="vertical" style={{ width: '100%' }} size={16}>
            <div className={styles.detailGrid}>
              <div><div className={styles.detailLabel}>区域</div><div className={styles.detailValue}>{zones.find(z => z.id === detailDrawer.area_id)?.name || detailDrawer.area_id}</div></div>
              <div><div className={styles.detailLabel}>日期</div><div className={styles.detailValue}>{detailDrawer.log_date}</div></div>
              <div><div className={styles.detailLabel}>时间</div><div className={styles.detailValue}>{detailDrawer.log_time || '--'}</div></div>
              <div><div className={styles.detailLabel}>类别</div><div className={styles.detailValue}><Tag>{detailDrawer.category}</Tag></div></div>
              <div><div className={styles.detailLabel}>操作人</div><div className={styles.detailValue}>{detailDrawer.operator || '--'}</div></div>
              <div><div className={styles.detailLabel}>天气</div><div className={styles.detailValue}>{detailDrawer.weather || '--'}</div></div>
            </div>

            <div className={styles.detailSection}>
              <div className={styles.detailSectionTitle}>操作内容</div>
              <div style={{ fontSize: 13 }}>{detailDrawer.content}</div>
            </div>

            {detailDrawer.crop_status && (
              <div className={styles.detailSection}>
                <div className={styles.detailSectionTitle}>作物状态</div>
                <div style={{ fontSize: 13 }}>{detailDrawer.crop_status}</div>
              </div>
            )}

            {/* Pesticide details */}
            {detailDrawer.category === '打药' && detailDrawer.details && (detailDrawer.details as any).items?.length > 0 && (
              <div className={styles.detailSection}>
                <div className={styles.detailSectionTitle}>
                  <BugOutlined /> 打药记录
                </div>
                <div className={styles.detailGrid}>
                  <div><div className={styles.detailLabel}>防治对象</div><div className={styles.detailValue}>{(detailDrawer.details as any).target_pest || '--'}</div></div>
                  <div><div className={styles.detailLabel}>施药方式</div><div className={styles.detailValue}>{(detailDrawer.details as any).application_method || '--'}</div></div>
                  <div><div className={styles.detailLabel}>用水量</div><div className={styles.detailValue}>{(detailDrawer.details as any).water_volume || '--'}</div></div>
                </div>
                <table className={styles.detailTable}>
                  <thead>
                    <tr>
                      <th>剂型</th><th>有效成分</th><th>品牌</th><th>登记证号</th><th>稀释倍数</th><th>亩用量</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(detailDrawer.details as any).items.map((item: any, i: number) => (
                      <tr key={i}>
                        <td>{item.formulation}</td>
                        <td>{item.ingredient}</td>
                        <td>{item.brand}</td>
                        <td style={{ fontFamily: 'monospace', fontSize: 11 }}>{item.reg_no}</td>
                        <td>{item.dosage}</td>
                        <td>{item.dosage_per_unit}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}

            {/* Fertilizer details */}
            {detailDrawer.category === '施肥' && detailDrawer.details && (detailDrawer.details as any).items?.length > 0 && (
              <div className={styles.detailSection}>
                <div className={styles.detailSectionTitle}>
                  <EnvironmentOutlined /> 施肥记录
                </div>
                <div className={styles.detailGrid}>
                  <div><div className={styles.detailLabel}>施肥方式</div><div className={styles.detailValue}>{(detailDrawer.details as any).method || '--'}</div></div>
                  <div><div className={styles.detailLabel}>总液量</div><div className={styles.detailValue}>{(detailDrawer.details as any).total_volume || '--'}</div></div>
                  <div><div className={styles.detailLabel}>EC</div><div className={styles.detailValue}>{(detailDrawer.details as any).ec || '--'}</div></div>
                  <div><div className={styles.detailLabel}>pH</div><div className={styles.detailValue}>{(detailDrawer.details as any).ph || '--'}</div></div>
                </div>
                <table className={styles.detailTable}>
                  <thead>
                    <tr>
                      <th>肥料名称</th><th>用量</th><th>N(%)</th><th>P(%)</th><th>K(%)</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(detailDrawer.details as any).items.map((item: any, i: number) => (
                      <tr key={i}>
                        <td>{item.name}</td>
                        <td>{item.amount}</td>
                        <td>{item.n}</td>
                        <td>{item.p}</td>
                        <td>{item.k}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}

            {detailDrawer.notes && (
              <div className={styles.detailSection}>
                <div className={styles.detailSectionTitle}>备注</div>
                <div style={{ fontSize: 13, color: '#595959' }}>{detailDrawer.notes}</div>
              </div>
            )}
          </Space>
        )}
      </Drawer>
    </div>
  );
};

export default FarmLog;
