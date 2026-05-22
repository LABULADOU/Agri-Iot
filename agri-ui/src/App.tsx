import React, { useEffect } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { ConfigProvider } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import { antdTheme } from './theme/antdConfig';
import { AppLayout } from './components/Layout';
import { useRealtimeStore } from './stores/realtimeStore';
import Dashboard from './pages/Dashboard';
import ZoneDetail from './pages/ZoneDetail';
import NodeList from './pages/NodeList';
import DataQuery from './pages/DataQuery';
import RuleList from './pages/RuleList';
import Settings from './pages/Settings';
import AIDecisions from './pages/AIDecisions';

const App: React.FC = () => {
  useEffect(() => {
    useRealtimeStore.getState().connect();
  }, []);

  return (
    <ConfigProvider locale={zhCN} theme={antdTheme}>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<AppLayout />}>
            <Route index element={<Dashboard />} />
            <Route path="zones/:id" element={<ZoneDetail />} />
            <Route path="nodes" element={<NodeList />} />
            <Route path="query" element={<DataQuery />} />
            <Route path="automation" element={<RuleList />} />
            <Route path="ai" element={<AIDecisions />} />
            <Route path="settings" element={<Settings />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </ConfigProvider>
  );
};

export default App;
