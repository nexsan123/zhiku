import { Component } from 'react';
import { useAppStore } from '@stores/app-store';
import { TitleBar } from '@components/title-bar';
import { Sidebar } from '@components/sidebar';
import { StatusBar } from '@components/status-bar';
import { MapPage } from '@components/map';
import { FinancePage } from '@components/finance';
import { AiChatPage } from '@components/ai-chat';
import { NotificationsPage } from '@components/notifications';
import { SettingsPage } from '@components/settings';

class ErrorBoundary extends Component<
  { children: React.ReactNode },
  { hasError: boolean; error: Error | null }
> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false, error: null };
  }
  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }
  render() {
    if (this.state.hasError) {
      return (
        <div style={{ padding: 40, color: '#FF453A', background: '#1a1a1a', fontFamily: 'monospace' }}>
          <h2>Page Render Error</h2>
          <pre style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-all' }}>
            {this.state.error?.message}
            {'\n\n'}
            {this.state.error?.stack}
          </pre>
        </div>
      );
    }
    return this.props.children;
  }
}

function App() {
  const currentPage = useAppStore((s) => s.currentPage);

  return (
    <div className="app">
      <TitleBar />

      <div className="app__body">
        <Sidebar />

        <main className="main-content">
          <ErrorBoundary>
            {currentPage === 'map' && <MapPage />}
            {currentPage === 'finance' && <FinancePage />}
            {currentPage === 'ai' && <AiChatPage />}
            {currentPage === 'notifications' && <NotificationsPage />}
            {currentPage === 'settings' && <SettingsPage />}
          </ErrorBoundary>
        </main>
      </div>

      <StatusBar />
    </div>
  );
}

export default App;
