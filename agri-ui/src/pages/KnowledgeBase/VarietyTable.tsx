import React, { useEffect, useState, useMemo, useRef, useCallback } from 'react';
import { Input, Table, Typography, Spin, Empty, Button, Space } from 'antd';
import { SearchOutlined, UpOutlined, DownOutlined } from '@ant-design/icons';
import { match } from 'pinyin-pro';
import { aiApi } from '../../services/api';
import type { ChrysanthemumVariety } from '../../types';
import styles from './VarietyTable.module.css';

const { Text } = Typography;

const FLOWER_TYPE_COLORS: Record<string, string> = {
  '重瓣': 'magenta',
  '单瓣': 'blue',
  '扣菊': 'purple',
  '桂瓣': 'orange',
  '迷你': 'cyan',
  '多头': 'green',
  '乒乓（单头）': 'geekblue',
  '丝线菊（单头）': 'lime',
  '重瓣（单头）': 'volcano',
  '管状': 'gold',
  '面包菊（单头）': 'red',
};

const VarietyTable: React.FC = () => {
  const [varieties, setVarieties] = useState<ChrysanthemumVariety[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [matchIdx, setMatchIdx] = useState(0);
  const tableRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setLoading(true);
    aiApi.chrysanthemumVarieties()
      .then(res => setVarieties(res.varieties))
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  const searchPredicate = useCallback((v: ChrysanthemumVariety, query: string) => {
    if (!query.trim()) return false;
    const q = query.trim().toLowerCase();
    if (v.name.toLowerCase().includes(q)) return true;
    if (v.color.includes(q)) return true;
    if (v.flower_type.includes(q)) return true;
    if (match(v.name, q, { precision: 'any', v: true })) return true;
    return false;
  }, []);

  const matchIndices = useMemo(() => {
    if (!search.trim()) return [];
    return varieties
      .map((v, i) => searchPredicate(v, search) ? i : -1)
      .filter(i => i >= 0);
  }, [search, varieties, searchPredicate]);

  useEffect(() => {
    setMatchIdx(0);
  }, [search]);

  const targetIndex = matchIndices.length > 0 ? matchIndices[matchIdx] ?? matchIndices[0] : -1;

  useEffect(() => {
    if (targetIndex < 0) return;
    requestAnimationFrame(() => {
      if (!tableRef.current) return;
      const el = tableRef.current.querySelector<HTMLElement>(`[data-row-key="${targetIndex}"]`);
      if (el) {
        el.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }
    });
  }, [targetIndex]);

  const handlePrev = useCallback(() => {
    setMatchIdx(i => i > 0 ? i - 1 : 0);
  }, []);

  const handleNext = useCallback(() => {
    setMatchIdx(i => i < matchIndices.length - 1 ? i + 1 : i);
  }, [matchIndices]);

  const columns = [
    {
      title: '品种',
      dataIndex: 'name',
      key: 'name',
      width: 180,
    },
    { title: '长势', dataIndex: 'growth', key: 'growth', width: 60 },
    { title: '到花周', dataIndex: 'weeks', key: 'weeks', width: 70 },
    { title: '颜色', dataIndex: 'color', key: 'color', width: 110 },
    {
      title: '花型',
      dataIndex: 'flower_type',
      key: 'flower_type',
      width: 120,
      render: (t: string) => (
        <span>
          {FLOWER_TYPE_COLORS[t] ? (
            <span style={{ color: FLOWER_TYPE_COLORS[t] }}>{t}</span>
          ) : t}
        </span>
      ),
    },
    {
      title: '耐低温',
      dataIndex: 'cold_tolerant',
      key: 'cold_tolerant',
      width: 70,
      render: (v: string) => v === '√' ? <span style={{ color: '#52c41a' }}>✓</span> : <span style={{ color: '#ff4d4f' }}>✗</span>,
    },
    {
      title: '耐高温',
      dataIndex: 'heat_tolerant',
      key: 'heat_tolerant',
      width: 70,
      render: (v: string) => v === '√' ? <span style={{ color: '#52c41a' }}>✓</span> : <span style={{ color: '#ff4d4f' }}>✗</span>,
    },
    {
      title: '抗病性',
      dataIndex: 'disease_resistance',
      key: 'disease_resistance',
      width: 80,
      render: (v: string) => {
        const color = v === '强' ? '#52c41a' : v === '弱' ? '#ff4d4f' : '#faad14';
        return <span style={{ color }}>{v}</span>;
      },
    },
  ];

  if (loading) {
    return <div className={styles.loading}><Spin size="large" /></div>;
  }

  const dataSource = varieties.map((v, i) => ({ ...v, key: i }));

  return (
    <div className={styles.container} ref={tableRef}>
      <Input
        className={styles.searchInput}
        placeholder="输入品种名或拼音首字母检索…"
        prefix={<SearchOutlined />}
        allowClear
        value={search}
        onChange={e => setSearch(e.target.value)}
        size="middle"
      />
      <div className={styles.navBar}>
        <Space>
          <Text type="secondary">
            共 {varieties.length} 个品种
          </Text>
          {matchIndices.length > 0 && (
            <>
              <Text type="secondary" style={{ marginLeft: 8 }}>·</Text>
              <Text type="secondary">
                找到 {matchIndices.length} 个匹配
              </Text>
              <Button
                size="small"
                icon={<UpOutlined />}
                disabled={matchIdx <= 0}
                onClick={handlePrev}
              >
                上一个
              </Button>
              <Text>
                {matchIdx + 1} / {matchIndices.length}
              </Text>
              <Button
                size="small"
                icon={<DownOutlined />}
                disabled={matchIdx >= matchIndices.length - 1}
                onClick={handleNext}
              >
                下一个
              </Button>
            </>
          )}
        </Space>
      </div>
      {!search.trim() && varieties.length === 0 && (
        <Empty description="暂无品种数据" image={Empty.PRESENTED_IMAGE_SIMPLE} />
      )}
      <Table
        columns={columns}
        dataSource={dataSource}
        loading={loading}
        size="small"
        pagination={false}
        scroll={{ x: 800, y: 600 }}
        rowClassName={(_record, index) => {
          return index === targetIndex ? styles.highlightRow : styles.normalRow;
        }}
        onRow={(_record, index) => ({
          'data-row-key': index,
        } as React.HTMLAttributes<HTMLElement>)}
      />
    </div>
  );
};

export default VarietyTable;
