import { useState, useEffect, useRef, lazy, Suspense } from 'react';
import ErrorBoundary from './components/ErrorBoundary';
import { IconSprout, IconLayoutDashboard, IconMap, IconBrainCircuit, IconBell, IconSettings } from './components/Icons';

const Overview = lazy(() => import('./pages/Overview'));
const ZoneDetail = lazy(() => import('./pages/ZoneDetail'));
const AIReview = lazy(() => import('./pages/AIReview'));
const Alerts = lazy(() => import('./pages/Alerts'));
const Settings = lazy(() => import('./pages/Settings'));

export function navigate(href) {
  window.history.pushState(null, '', href);
  window.dispatchEvent(new PopStateEvent('popstate'));
}

function usePath() {
  const [path, setPath] = useState(window.location.pathname);
  useEffect(() => {
    const handler = () => setPath(window.location.pathname);
    window.addEventListener('popstate', handler);
    return () => window.removeEventListener('popstate', handler);
  }, []);
  return path;
}

const navItems = [
  { label: '全局态势', icon: IconLayoutDashboard, href: '/' },
  { label: '区域孪生', icon: IconMap, href: '/zone' },
  { label: 'AI 决策', icon: IconBrainCircuit, href: '/ai' },
  { label: '告警中心', icon: IconBell, href: '/alerts' },
  { label: '系统设置', icon: IconSettings, href: '/settings' },
];

function Spinner() {
  return (
    <div className="container" style={{ textAlign: 'center', padding: 60 }}>
      <div className="pulse-dot" style={{ margin: '0 auto', width: 12, height: 12 }} />
    </div>
  );
}

function NavButton({ item, path, onClick }) {
  const Icon = item.icon;
  const active = path === item.href;
  return (
    <button
      className={`sidebar-nav-item${active ? ' active' : ''}`}
      onClick={onClick}
      aria-current={active ? 'page' : undefined}
    >
      <Icon size={18} />
      <span>{item.label}</span>
    </button>
  );
}

function MobileTab({ item, path, onClick }) {
  const Icon = item.icon;
  const active = path === item.href;
  return (
    <button
      className={`mobile-tab-item${active ? ' active' : ''}`}
      onClick={onClick}
      aria-current={active ? 'page' : undefined}
    >
      <Icon size={20} />
      <span>{item.label}</span>
    </button>
  );
}

export default function App() {
  const [mobileOpen, setMobileOpen] = useState(false);
  const path = usePath();
  const mainRef = useRef(null);

  let page;
  const pageLabel = {
    '/': '全局态势感知',
    '/zone': '区域孪生',
    '/ai': 'AI 决策与复盘',
    '/alerts': '告警中心',
    '/settings': '系统设置',
  };
  const currentLabel = pageLabel[path] || '全局态势感知';

  switch (path) {
    case '/': page = <ErrorBoundary key="ov"><Overview /></ErrorBoundary>; break;
    case '/zone': page = <ErrorBoundary key="zd"><ZoneDetail /></ErrorBoundary>; break;
    case '/ai': page = <ErrorBoundary key="ai"><AIReview /></ErrorBoundary>; break;
    case '/alerts': page = <ErrorBoundary key="al"><Alerts /></ErrorBoundary>; break;
    case '/settings': page = <ErrorBoundary key="st"><Settings /></ErrorBoundary>; break;
    default: page = <ErrorBoundary key="ov"><Overview /></ErrorBoundary>;
  }

  useEffect(() => {
    if (mainRef.current) {
      const heading = mainRef.current.querySelector('h2');
      if (heading) heading.focus({ preventScroll: true });
      else mainRef.current.focus({ preventScroll: true });
    }
  }, [path]);

  const handleNav = (href) => {
    navigate(href);
    setMobileOpen(false);
  };

  return (
    <div className="app-shell">
      <a href="#main-content" className="skip-link">跳到主内容</a>

      <div className="sr-only" role="status" aria-live="polite" aria-atomic="true">
        {currentLabel} 页面已加载
      </div>

      <nav className="sidebar" role="navigation" aria-label="主导航">
        <div className="sidebar-brand">
          <div className="sidebar-brand-icon"><IconSprout size={18} /></div>
          <div>
            <h3>Agri-IoT</h3>
            <small>智慧农业监控</small>
          </div>
        </div>
        <div className="sidebar-nav">
          {navItems.map(item => (
            <NavButton key={item.href} item={item} path={path} onClick={() => handleNav(item.href)} />
          ))}
        </div>
        <div className="sidebar-footer">v0.2 · 数字孪生控制台</div>
      </nav>

      <nav className="mobile-tabbar" role="navigation" aria-label="移动端导航">
        {navItems.map(item => (
          <MobileTab key={item.href} item={item} path={path} onClick={() => handleNav(item.href)} />
        ))}
      </nav>

      <main className="main-area" id="main-content" role="main" aria-label={currentLabel} ref={mainRef} tabIndex={-1}>
        <Suspense fallback={<Spinner />}>{page}</Suspense>
      </main>
    </div>
  );
}
