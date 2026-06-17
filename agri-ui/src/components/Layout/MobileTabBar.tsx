import React from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import {
  DashboardOutlined,
  DatabaseOutlined,
  SettingOutlined,
  RobotOutlined,
  ReadOutlined,
} from '@ant-design/icons';
import styles from './MobileTabBar.module.css';

const tabs = [
  { key: '/', icon: <DashboardOutlined />, label: '总览' },
  { key: '/query', icon: <DatabaseOutlined />, label: '数据' },
  { key: '/ai', icon: <RobotOutlined />, label: 'AI' },
  { key: '/knowledge', icon: <ReadOutlined />, label: '知识' },
  { key: '/settings', icon: <SettingOutlined />, label: '设置' },
];

const MobileTabBar: React.FC = () => {
  const navigate = useNavigate();
  const location = useLocation();

  const selectedKey = tabs.find(item =>
    location.pathname === item.key || location.pathname.startsWith(item.key + '/')
  )?.key || '/';

  return (
    <nav className={styles.tabBar}>
      {tabs.map(tab => (
        <button
          key={tab.key}
          className={`${styles.tab} ${selectedKey === tab.key ? styles.active : ''}`}
          onClick={() => navigate(tab.key)}
        >
          <span className={styles.tabIcon}>{tab.icon}</span>
          <span className={styles.tabLabel}>{tab.label}</span>
        </button>
      ))}
    </nav>
  );
};

export default MobileTabBar;
