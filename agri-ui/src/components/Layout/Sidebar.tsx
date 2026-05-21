import React, { useState } from 'react';
import { Layout, Menu, Tooltip } from 'antd';
import { useNavigate, useLocation } from 'react-router-dom';
import {
  DashboardOutlined,
  AppstoreOutlined,
  DatabaseOutlined,
  SettingOutlined,
  AlertOutlined,
  RobotOutlined,
} from '@ant-design/icons';
import styles from './Sidebar.module.css';

const { Sider } = Layout;

interface MenuItem {
  key: string;
  icon: React.ReactNode;
  label: string;
}

const menuItems: MenuItem[] = [
  { key: '/', icon: <DashboardOutlined />, label: '总览' },
  { key: '/zones', icon: <AppstoreOutlined />, label: '区域' },
  { key: '/query', icon: <DatabaseOutlined />, label: '数据' },
  { key: '/ai', icon: <RobotOutlined />, label: 'AI 决策' },
  { key: '/automation', icon: <AlertOutlined />, label: '自动化' },
  { key: '/settings', icon: <SettingOutlined />, label: '设置' },
];

const Sidebar: React.FC = () => {
  const navigate = useNavigate();
  const location = useLocation();
  const [collapsed, setCollapsed] = useState(true);

  const selectedKey = menuItems.find(item =>
    location.pathname === item.key || location.pathname.startsWith(item.key + '/')
  )?.key || '/';

  return (
    <Sider
      width={200}
      collapsedWidth={56}
      collapsed={collapsed}
      onMouseEnter={() => setCollapsed(false)}
      onMouseLeave={() => setCollapsed(true)}
      className={styles.sider}
    >
      <div className={styles.logo}>{collapsed ? '🌱' : '🌱 Agri-IoT'}</div>
      <Menu
        mode="inline"
        selectedKeys={[selectedKey]}
        items={menuItems.map(item => ({
          key: item.key,
          icon: collapsed ? (
            <Tooltip placement="right" title={item.label}>
              {item.icon}
            </Tooltip>
          ) : item.icon,
          label: item.label,
        }))}
        onClick={({ key }) => navigate(key)}
        className={styles.menu}
      />
    </Sider>
  );
};

export default Sidebar;
