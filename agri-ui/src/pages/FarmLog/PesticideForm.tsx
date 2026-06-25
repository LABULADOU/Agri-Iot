import React from 'react';
import { Form, Input, Select, Button, Space } from 'antd';
import { PlusOutlined, DeleteOutlined } from '@ant-design/icons';

const { TextArea } = Input;

interface PesticideItem {
  formulation: string;
  ingredient: string;
  brand: string;
  reg_no: string;
  dosage: string;
  dosage_per_unit: string;
}

interface PesticideFormProps {
  value?: { items: PesticideItem[]; water_volume: string; target_pest: string; application_method: string };
  onChange?: (val: Record<string, unknown>) => void;
}

const formulationOptions = [
  { value: '悬浮剂', label: '悬浮剂' },
  { value: '水剂', label: '水剂' },
  { value: '乳油', label: '乳油' },
  { value: '可湿性粉剂', label: '可湿性粉剂' },
  { value: '水分散粒剂', label: '水分散粒剂' },
  { value: '颗粒剂', label: '颗粒剂' },
  { value: '烟剂', label: '烟剂' },
];

const methodOptions = [
  { value: '叶面喷雾', label: '叶面喷雾' },
  { value: '灌根', label: '灌根' },
  { value: '熏蒸', label: '熏蒸' },
  { value: '撒施', label: '撒施' },
  { value: '涂抹', label: '涂抹' },
];

const PesticideForm: React.FC<PesticideFormProps> = ({ value, onChange }) => {
  const items = value?.items || [];
  const water_volume = value?.water_volume || '';
  const target_pest = value?.target_pest || '';
  const application_method = value?.application_method || '';

  const update = (patch: Partial<Record<string, unknown>>) => {
    onChange?.({ items, water_volume, target_pest, application_method, ...patch });
  };

  const updateItem = (i: number, patch: Partial<PesticideItem>) => {
    const next = [...items];
    next[i] = { ...next[i], ...patch };
    update({ items: next });
  };

  const addItem = () => {
    update({ items: [...items, { formulation: '', ingredient: '', brand: '', reg_no: '', dosage: '', dosage_per_unit: '' }] });
  };

  const removeItem = (i: number) => {
    update({ items: items.filter((_, idx) => idx !== i) });
  };

  return (
    <Space direction="vertical" style={{ width: '100%' }} size={12}>
      <Space style={{ width: '100%' }}>
        <Form.Item label="防治对象" style={{ marginBottom: 0, flex: 1 }}>
          <Input value={target_pest} onChange={e => update({ target_pest: e.target.value })} placeholder="如：灰霉病" />
        </Form.Item>
        <Form.Item label="施药方式" style={{ marginBottom: 0, width: 140 }}>
          <Select value={application_method} onChange={v => update({ application_method: v })} options={methodOptions} placeholder="选择方式" allowClear />
        </Form.Item>
        <Form.Item label="用水量" style={{ marginBottom: 0, width: 120 }}>
          <Input value={water_volume} onChange={e => update({ water_volume: e.target.value })} placeholder="如：30L" />
        </Form.Item>
      </Space>

      <div style={{ fontSize: 13, fontWeight: 600, color: '#595959' }}>药剂清单</div>

      {items.map((item, i) => (
        <Space key={i} style={{ width: '100%' }} align="start" wrap>
          <Form.Item label="剂型" style={{ marginBottom: 0, width: 130 }}>
            <Select value={item.formulation} onChange={v => updateItem(i, { formulation: v })} options={formulationOptions} placeholder="剂型" size="small" style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item label="有效成分" style={{ marginBottom: 0, width: 100 }}>
            <Input value={item.ingredient} onChange={e => updateItem(i, { ingredient: e.target.value })} placeholder="如：嘧霉胺" size="small" />
          </Form.Item>
          <Form.Item label="品牌" style={{ marginBottom: 0, width: 90 }}>
            <Input value={item.brand} onChange={e => updateItem(i, { brand: e.target.value })} placeholder="品牌" size="small" />
          </Form.Item>
          <Form.Item label="登记证号" style={{ marginBottom: 0, width: 120 }}>
            <Input value={item.reg_no} onChange={e => updateItem(i, { reg_no: e.target.value })} placeholder="如：PD2020XXXX" size="small" />
          </Form.Item>
          <Form.Item label="稀释倍数" style={{ marginBottom: 0, width: 100 }}>
            <Input value={item.dosage} onChange={e => updateItem(i, { dosage: e.target.value })} placeholder="如：1500倍液" size="small" />
          </Form.Item>
          <Form.Item label="亩用量" style={{ marginBottom: 0, width: 80 }}>
            <Input value={item.dosage_per_unit} onChange={e => updateItem(i, { dosage_per_unit: e.target.value })} placeholder="如：30ml" size="small" />
          </Form.Item>
          <Button type="text" danger icon={<DeleteOutlined />} onClick={() => removeItem(i)} style={{ marginTop: 28 }} />
        </Space>
      ))}

      <Button type="dashed" onClick={addItem} icon={<PlusOutlined />} size="small" style={{ width: 200 }}>
        添加药剂
      </Button>
    </Space>
  );
};

export default PesticideForm;
