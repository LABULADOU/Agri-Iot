import React, { useEffect } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { ConfigProvider } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import { antdTheme } from './theme/antdConfig';
import ErrorBoundary from './components/common/ErrorBoundary';
import { AppLayout } from './components/Layout';
import { useRealtimeStore } from './stores/realtimeStore';
import Dashboard from './pages/Dashboard';
import ZoneDetail from './pages/ZoneDetail';
import NodeList from './pages/NodeList';
import DataQuery from './pages/DataQuery';
import AI from './pages/AI';
import KnowledgeBase from './pages/KnowledgeBase';
import Settings from './pages/Settings';
import FarmLog from './pages/FarmLog';

const App: React.FC = () => {
  useEffect(() => {
    useRealtimeStore.getState().connect();
  }, []);

  return (
    <ConfigProvider locale={zhCN} theme={antdTheme}>
      <ErrorBoundary>
        <BrowserRouter>
          <Routes>
            <Route path="/" element={<AppLayout />}>
              <Route index element={<Dashboard />} />
              <Route path="zones/:id" element={<ZoneDetail />} />
              <Route path="nodes" element={<NodeList />} />
              <Route path="query" element={<DataQuery />} />
              <Route path="ai" element={<AI />} />
              <Route path="knowledge" element={<KnowledgeBase />} />
              <Route path="farm-logs" element={<FarmLog />} />
              <Route path="settings" element={<Settings />} />
              <Route path="automation" element={<Navigate to="/settings?tab=rules" replace />} />
              <Route path="agent" element={<Navigate to="/ai?tab=chat" replace />} />
              <Route path="*" element={<Navigate to="/" replace />} />
            </Route>
          </Routes>
        </BrowserRouter>
      </ErrorBoundary>
    </ConfigProvider>
  );
};

export default App;
