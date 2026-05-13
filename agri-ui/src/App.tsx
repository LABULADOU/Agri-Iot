import React from 'react';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { ConfigProvider } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import { MainLayout } from './components/Layout';
import Dashboard from './pages/Dashboard';
import ZoneList from './pages/ZoneList';
import ZoneDetail from './pages/ZoneDetail';
import NodeList from './pages/NodeList';
import DataQuery from './pages/DataQuery';
import RuleList from './pages/RuleList';
import Settings from './pages/Settings';

const App: React.FC = () => {
  return (
    <ConfigProvider locale={zhCN}>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<MainLayout />}>
            <Route index element={<Dashboard />} />
            <Route path="zones" element={<ZoneList />} />
            <Route path="zones/:id" element={<ZoneDetail />} />
            <Route path="nodes" element={<NodeList />} />
            <Route path="query" element={<DataQuery />} />
            <Route path="rules" element={<RuleList />} />
            <Route path="settings" element={<Settings />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </ConfigProvider>
  );
};

export default App;