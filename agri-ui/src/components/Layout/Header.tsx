import React from 'react';
import { Layout, Space, Typography, Badge } from 'antd';
import { WifiOutlined, ClockCircleOutlined } from '@ant-design/icons';
import { useRealtimeStore } from '../../stores/realtimeStore';
import styles from './Header.module.css';

const { Header: AntHeader } = Layout;
const { Text } = Typography;

const Header: React.FC = () => {
  const { connected, lastUpdate } = useRealtimeStore();

  return (
    <AntHeader className={styles.header}>
      <div />
      <Space size="large">
        {lastUpdate && (
          <Space size={4}>
            <ClockCircleOutlined />
            <Text type="secondary" className={styles.time}>
              {new Date(lastUpdate).toLocaleTimeString('zh-CN')}
            </Text>
          </Space>
        )}
        <Badge status={connected ? 'success' : 'error'} text={
          <Space size={4}>
            <WifiOutlined />
            <Text type="secondary">{connected ? '已连接' : '未连接'}</Text>
          </Space>
        } />
      </Space>
    </AntHeader>
  );
};

export default Header;