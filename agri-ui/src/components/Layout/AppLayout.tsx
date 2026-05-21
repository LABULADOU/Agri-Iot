import React from 'react';
import { Layout } from 'antd';
import { Outlet } from 'react-router-dom';
import Sidebar from './Sidebar';
import TopBar from './TopBar';
import styles from './AppLayout.module.css';

const { Content } = Layout;

const AppLayout: React.FC = () => {
  return (
    <Layout className={styles.layout}>
      <Sidebar />
      <Layout>
        <TopBar />
        <Content className={styles.content}>
          <Outlet />
        </Content>
      </Layout>
    </Layout>
  );
};

export default AppLayout;
