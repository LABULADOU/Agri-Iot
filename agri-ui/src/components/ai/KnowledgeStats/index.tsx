import React from 'react';
import { Typography } from 'antd';
import styles from './KnowledgeStats.module.css';

const { Text } = Typography;

interface KnowledgeStatsProps {
  cropCount: number;
  pestCount: number;
  caseCount: number;
  thisMonthNew: number;
}

const KnowledgeStats: React.FC<KnowledgeStatsProps> = ({ cropCount, pestCount, caseCount, thisMonthNew }) => {
  const items = [
    { label: '作物', value: cropCount },
    { label: '病虫害', value: pestCount },
    { label: '案例', value: caseCount },
    { label: '本月新增', value: thisMonthNew },
  ];

  return (
    <div className={styles.grid}>
      {items.map(item => (
        <div key={item.label} className={styles.item}>
          <Text className={styles.value}>{item.value}</Text>
          <Text type="secondary" className={styles.label}>{item.label}</Text>
        </div>
      ))}
    </div>
  );
};

export default KnowledgeStats;
