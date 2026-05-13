import React from 'react';
import { Layout } from 'antd';
import { Outlet } from 'react-router-dom';
import Sidebar from './Sidebar';
import Header from './Header';
import styles from './MainLayout.module.css';

const { Content } = Layout;

const MainLayout: React.FC = () => {
  return (
    <Layout className={styles.layout}>
      <Sidebar />
      <Layout>
        <Header />
        <Content className={styles.content}>
          <Outlet />
        </Content>
      </Layout>
    </Layout>
  );
};

export default MainLayout;