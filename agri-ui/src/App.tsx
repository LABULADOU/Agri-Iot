import React, { lazy, Suspense, useEffect } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { ConfigProvider, Spin } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import { antdTheme } from './theme/antdConfig';
import ErrorBoundary from './components/common/ErrorBoundary';
import { AppLayout } from './components/Layout';
import { useRealtimeStore } from './stores/realtimeStore';

const Dashboard = lazy(() => import('./pages/Dashboard'));
const ZoneDetail = lazy(() => import('./pages/ZoneDetail'));
const NodeList = lazy(() => import('./pages/NodeList'));
const DataQuery = lazy(() => import('./pages/DataQuery'));
const AI = lazy(() => import('./pages/AI'));
const KnowledgeBase = lazy(() => import('./pages/KnowledgeBase'));
const Settings = lazy(() => import('./pages/Settings'));
const FarmLog = lazy(() => import('./pages/FarmLog'));
const Inventory = lazy(() => import('./pages/Inventory'));
const Yield = lazy(() => import('./pages/Yield'));
const Mixing = lazy(() => import('./pages/Mixing'));
const Labor = lazy(() => import('./pages/Labor'));

const loadingFallback = (
  <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: '60vh' }}>
    <Spin tip="加载中..." />
  </div>
);

const withBoundary = (node: React.ReactNode) => <ErrorBoundary>{node}</ErrorBoundary>;

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
              <Route index element={withBoundary(<Suspense fallback={loadingFallback}><Dashboard /></Suspense>)} />
              <Route path="zones/:id" element={withBoundary(<Suspense fallback={loadingFallback}><ZoneDetail /></Suspense>)} />
              <Route path="nodes" element={withBoundary(<Suspense fallback={loadingFallback}><NodeList /></Suspense>)} />
              <Route path="query" element={withBoundary(<Suspense fallback={loadingFallback}><DataQuery /></Suspense>)} />
              <Route path="ai" element={withBoundary(<Suspense fallback={loadingFallback}><AI /></Suspense>)} />
              <Route path="knowledge" element={withBoundary(<Suspense fallback={loadingFallback}><KnowledgeBase /></Suspense>)} />
              <Route path="farm-logs" element={withBoundary(<Suspense fallback={loadingFallback}><FarmLog /></Suspense>)} />
              <Route path="inventory" element={withBoundary(<Suspense fallback={loadingFallback}><Inventory /></Suspense>)} />
              <Route path="yield" element={withBoundary(<Suspense fallback={loadingFallback}><Yield /></Suspense>)} />
              <Route path="mixing" element={withBoundary(<Suspense fallback={loadingFallback}><Mixing /></Suspense>)} />
              <Route path="labor" element={withBoundary(<Suspense fallback={loadingFallback}><Labor /></Suspense>)} />
              <Route path="settings" element={withBoundary(<Suspense fallback={loadingFallback}><Settings /></Suspense>)} />
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
