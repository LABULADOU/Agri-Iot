import React from 'react';
import { Form, Input, Select, Button, Space } from 'antd';
import { PlusOutlined, DeleteOutlined } from '@ant-design/icons';

interface FertilizerItem {
  name: string;
  amount: string;
  n: string;
  p: string;
  k: string;
}

interface FertilizerFormProps {
  value?: { items: FertilizerItem[]; method: string; total_volume: string; ec: string; ph: string };
  onChange?: (val: Record<string, unknown>) => void;
}

const methodOptions = [
  { value: '滴灌', label: '滴灌' },
  { value: '冲施', label: '冲施' },
  { value: '沟施', label: '沟施' },
  { value: '穴施', label: '穴施' },
  { value: '撒施', label: '撒施' },
  { value: '叶面喷施', label: '叶面喷施' },
];

const FertilizerForm: React.FC<FertilizerFormProps> = ({ value, onChange }) => {
  const items = value?.items || [];
  const method = value?.method || '';
  const total_volume = value?.total_volume || '';
  const ec = value?.ec || '';
  const ph = value?.ph || '';

  const update = (patch: Partial<Record<string, unknown>>) => {
    onChange?.({ items, method, total_volume, ec, ph, ...patch });
  };

  const updateItem = (i: number, patch: Partial<FertilizerItem>) => {
    const next = [...items];
    next[i] = { ...next[i], ...patch };
    update({ items: next });
  };

  const addItem = () => {
    update({ items: [...items, { name: '', amount: '', n: '', p: '', k: '' }] });
  };

  const removeItem = (i: number) => {
    update({ items: items.filter((_, idx) => idx !== i) });
  };

  return (
    <Space direction="vertical" style={{ width: '100%' }} size={12}>
      <Space style={{ width: '100%' }}>
        <Form.Item label="施肥方式" style={{ marginBottom: 0, width: 130 }}>
          <Select value={method} onChange={v => update({ method: v })} options={methodOptions} placeholder="选择方式" allowClear />
        </Form.Item>
        <Form.Item label="总液量" style={{ marginBottom: 0, width: 120 }}>
          <Input value={total_volume} onChange={e => update({ total_volume: e.target.value })} placeholder="如：200L" />
        </Form.Item>
        <Form.Item label="EC" style={{ marginBottom: 0, width: 100 }}>
          <Input value={ec} onChange={e => update({ ec: e.target.value })} placeholder="如：1.2" />
        </Form.Item>
        <Form.Item label="pH" style={{ marginBottom: 0, width: 100 }}>
          <Input value={ph} onChange={e => update({ ph: e.target.value })} placeholder="如：6.0" />
        </Form.Item>
      </Space>

      <div style={{ fontSize: 13, fontWeight: 600, color: '#595959' }}>肥料清单</div>

      {items.map((item, i) => (
        <Space key={i} style={{ width: '100%' }} align="start" wrap>
          <Form.Item label="肥料名称" style={{ marginBottom: 0, width: 130 }}>
            <Input value={item.name} onChange={e => updateItem(i, { name: e.target.value })} placeholder="如：硝酸钙" size="small" style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item label="用量" style={{ marginBottom: 0, width: 110 }}>
            <Input value={item.amount} onChange={e => updateItem(i, { amount: e.target.value })} placeholder="如：300g/株" size="small" />
          </Form.Item>
          <Form.Item label="N(%)" style={{ marginBottom: 0, width: 70 }}>
            <Input value={item.n} onChange={e => updateItem(i, { n: e.target.value })} placeholder="15.5" size="small" />
          </Form.Item>
          <Form.Item label="P(%)" style={{ marginBottom: 0, width: 70 }}>
            <Input value={item.p} onChange={e => updateItem(i, { p: e.target.value })} placeholder="0" size="small" />
          </Form.Item>
          <Form.Item label="K(%)" style={{ marginBottom: 0, width: 70 }}>
            <Input value={item.k} onChange={e => updateItem(i, { k: e.target.value })} placeholder="0" size="small" />
          </Form.Item>
          <Button type="text" danger icon={<DeleteOutlined />} onClick={() => removeItem(i)} style={{ marginTop: 28 }} />
        </Space>
      ))}

      <Button type="dashed" onClick={addItem} icon={<PlusOutlined />} size="small" style={{ width: 200 }}>
        添加肥料
      </Button>
    </Space>
  );
};

export default FertilizerForm;
