import React from 'react';
import { Typography, Tag } from 'antd';
import styles from './EmergencyRules.module.css';

const { Text } = Typography;

interface EmergencyRule {
  id: string;
  name: string;
  condition: string;
  active: boolean;
  action: string;
}

interface EmergencyRulesProps {
  rules: EmergencyRule[];
}

const EmergencyRules: React.FC<EmergencyRulesProps> = ({ rules }) => {
  return (
    <div className={styles.table}>
      <div className={styles.headRow}>
        <Text className={styles.th}>规则名</Text>
        <Text className={styles.th}>触发条件</Text>
        <Text className={styles.th}>当前状态</Text>
        <Text className={styles.th}>执行动作</Text>
      </div>
      {rules.map(r => (
        <div key={r.id} className={`${styles.row} ${r.active ? styles.active : ''}`}>
          <Text className={styles.td}>{r.name}</Text>
          <Text className={styles.td}>{r.condition}</Text>
          <Tag color={r.active ? 'red' : 'default'}>{r.active ? '已触发' : '正常'}</Tag>
          <Text className={styles.td}>{r.action}</Text>
        </div>
      ))}
    </div>
  );
};

export default EmergencyRules;
