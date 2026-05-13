import React from 'react';
import { Layout, Menu } from 'antd';
import { useNavigate, useLocation } from 'react-router-dom';
import {
  DashboardOutlined,
  AppstoreOutlined,
  DatabaseOutlined,
  SettingOutlined,
  ExperimentOutlined,
  AlertOutlined,
} from '@ant-design/icons';
import styles from './Sidebar.module.css';

const { Sider } = Layout;

const menuItems = [
  { key: '/', icon: <DashboardOutlined />, label: '总览' },
  { key: '/zones', icon: <AppstoreOutlined />, label: '区域管理' },
  { key: '/nodes', icon: <ExperimentOutlined />, label: '采集节点' },
  { key: '/query', icon: <DatabaseOutlined />, label: '数据查询' },
  { key: '/rules', icon: <AlertOutlined />, label: '规则管理' },
  { key: '/settings', icon: <SettingOutlined />, label: '系统设置' },
];

const Sidebar: React.FC = () => {
  const navigate = useNavigate();
  const location = useLocation();

  const selectedKey = menuItems.find(item =>
    location.pathname === item.key || location.pathname.startsWith(item.key + '/')
  )?.key || '/';

  return (
    <Sider width={220} className={styles.sider}>
      <div className={styles.logo}>
        <span>🌱 Agri-IoT</span>
      </div>
      <Menu
        mode="inline"
        selectedKeys={[selectedKey]}
        items={menuItems}
        onClick={({ key }: { key: string }) => navigate(key)}
        className={styles.menu}
      />
    </Sider>
  );
};

export default Sidebar;