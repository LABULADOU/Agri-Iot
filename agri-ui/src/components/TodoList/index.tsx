import React from 'react';
import { Typography, Button, Tag } from 'antd';
import type { TodoItem } from '../../types';
import styles from './TodoList.module.css';

const { Text } = Typography;

interface TodoListProps {
  items: TodoItem[];
  onExecute?: (item: TodoItem) => void;
}

const typeConfig: Record<string, { label: string; color: string }> = {
  warning: { label: '警告', color: 'red' },
  attention: { label: '注意', color: 'orange' },
  offline: { label: '离线', color: 'default' },
};

const TodoList: React.FC<TodoListProps> = ({ items, onExecute }) => {
  if (!items.length) {
    return (
      <div className={styles.empty}>
        <Text type="secondary">暂无待处理事项</Text>
      </div>
    );
  }

  return (
    <div className={styles.list}>
      {items.map(item => (
        <div key={item.id} className={styles.item}>
          <Tag color={typeConfig[item.type]?.color}>{typeConfig[item.type]?.label}</Tag>
          <Text strong className={styles.zone}>{item.zoneName}</Text>
          <Text className={styles.message}>{item.message}</Text>
          {item.aiRecommendation && (
            <Text type="secondary" className={styles.ai}>{item.aiRecommendation}</Text>
          )}
          <Text type="secondary" className={styles.time}>{item.timestamp}</Text>
          <Button
            size="small"
            type={item.actionable ? 'primary' : 'default'}
            onClick={() => onExecute?.(item)}
          >
            {item.actionable ? '执行' : '现场处理'}
          </Button>
        </div>
      ))}
    </div>
  );
};

export default TodoList;
