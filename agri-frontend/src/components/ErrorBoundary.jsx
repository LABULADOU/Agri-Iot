import { Component } from 'react';
import { IconAlertTriangle } from './Icons';

export default class ErrorBoundary extends Component {
  constructor(props) {
    super(props);
    this.state = { error: null };
  }

  static getDerivedStateFromError(error) {
    return { error };
  }

  render() {
    if (this.state.error) {
      return (
        <div className="container" style={{ textAlign: 'center', padding: 60 }}>
          <div style={{ marginBottom: 16 }}><IconAlertTriangle size={48} style={{ color: 'var(--red)' }} /></div>
          <h3 style={{ color: 'var(--red)', marginBottom: 8 }}>页面渲染异常</h3>
          <p className="text-dim text-sm" style={{ marginBottom: 20 }}>
            {this.state.error.message}
          </p>
          <button className="btn btn-sm" onClick={() => {
            this.setState({ error: null });
            window.history.pushState(null, '', '/');
            window.dispatchEvent(new PopStateEvent('popstate'));
          }}>
            返回首页
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
