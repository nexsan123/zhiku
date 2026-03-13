import { Component, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Newspaper,
  Activity,
  BarChart3,
  DollarSign,
  Droplets,
  Bitcoin,
  TrendingUp,
  Globe,
  Landmark,
  Link2,
  Building2,
  Compass,
} from 'lucide-react';
import { useAppStore } from '@stores/app-store';
import { TitleBar } from '@components/title-bar';
import { StatusBar } from '@components/status-bar';
import { PanelStack } from '@components/panel-stack';
import { Panel } from '@components/panel';
import { MapCenter } from '@components/map-center';
import { NewsFeedPanel } from '@components/panels/NewsFeedPanel';
import { FredPanel } from '@components/panels/FredPanel';
import { BisPanel } from '@components/panels/BisPanel';
import { SituationCenterPanel } from '@components/panels/SituationCenterPanel';
import { MarketRadarPanel } from '@components/panels/MarketRadarPanel';
import { IndicesPanel } from '@components/panels/IndicesPanel';
import { ForexPanel } from '@components/panels/ForexPanel';
import { OilEnergyPanel } from '@components/panels/OilEnergyPanel';
import { CryptoPanel } from '@components/panels/CryptoPanel';
import { FearGreedPanel } from '@components/panels/FearGreedPanel';
import { WtoPanel } from '@components/panels/WtoPanel';
import { SupplyChainPanel } from '@components/panels/SupplyChainPanel';
import { GulfFdiPanel } from '@components/panels/GulfFdiPanel';
import { CmdKModal } from '@components/cmd-k';
import { SettingsPage } from './components/settings/SettingsPage';
import { listenApiStatusChanged } from '@services/tauri-bridge';
import i18n from './i18n';

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
        <div
          style={{
            padding: 40,
            color: 'var(--color-semantic-error)',
            background: 'var(--color-bg-elevated)',
            fontFamily: 'var(--font-mono)',
          }}
        >
          <h2>{i18n.t('state.renderError')}</h2>
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
  const { t } = useTranslation();
  const leftPanelCollapsed = useAppStore((s) => s.leftPanelCollapsed);
  const rightPanelCollapsed = useAppStore((s) => s.rightPanelCollapsed);
  const toggleLeftPanel = useAppStore((s) => s.toggleLeftPanel);
  const toggleRightPanel = useAppStore((s) => s.toggleRightPanel);
  const updateApiStatus = useAppStore((s) => s.updateApiStatus);
  const cmdKOpen = useAppStore((s) => s.cmdKOpen);
  const setCmdKOpen = useAppStore((s) => s.setCmdKOpen);
  const settingsOpen = useAppStore((s) => s.settingsOpen);
  const closeSettings = useAppStore((s) => s.closeSettings);
  const settingsInitialTab = useAppStore((s) => s.settingsInitialTab);

  // ---- Ctrl+[ / Ctrl+] keyboard shortcuts for panel collapse + Cmd/Ctrl+K for search ----
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === '[') {
        e.preventDefault();
        toggleLeftPanel();
      }
      if (e.ctrlKey && e.key === ']') {
        e.preventDefault();
        toggleRightPanel();
      }
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setCmdKOpen(!cmdKOpen);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [toggleLeftPanel, toggleRightPanel, setCmdKOpen, cmdKOpen]);

  // ---- Listen for Tauri 'api-status-changed' events ----
  useEffect(() => {
    let cleanup: (() => void) | null = null;
    const unlistenPromise = listenApiStatusChanged((status) => {
      updateApiStatus(status.service, status);
    });
    void unlistenPromise.then((fn) => {
      cleanup = fn;
    });
    return () => {
      if (cleanup) {
        cleanup();
      } else {
        void unlistenPromise.then((fn) => fn());
      }
    };
  }, [updateApiStatus]);

  return (
    <div className="app">
      <TitleBar />

      <div className="app__body">
        {/* Left Panel Stack — 态势中枢 (L1), News Feed (L1), FRED Indicators (L2), BIS Rates (L2) */}
        <PanelStack side="left" collapsed={leftPanelCollapsed}>
          <Panel title={t('panel.situationCenter')} icon={<Compass size={13} />} panelId="situation-center">
            <ErrorBoundary>
              <SituationCenterPanel />
            </ErrorBoundary>
          </Panel>
          <Panel title={t('panel.newsFeed')} icon={<Newspaper size={13} />} panelId="news-feed">
            <ErrorBoundary>
              <NewsFeedPanel />
            </ErrorBoundary>
          </Panel>
          <Panel title={t('panel.fredIndicators')} icon={<Activity size={13} />} panelId="fred-indicators">
            <ErrorBoundary>
              <FredPanel />
            </ErrorBoundary>
          </Panel>
          <Panel title={t('panel.bisRates')} icon={<Landmark size={13} />} panelId="bis-rates">
            <ErrorBoundary>
              <BisPanel />
            </ErrorBoundary>
          </Panel>
        </PanelStack>

        {/* Center — Map placeholder (Phase 5: deck.gl) */}
        <ErrorBoundary>
          <MapCenter />
        </ErrorBoundary>

        {/* Right Panel Stack — Market Radar (L1), Indices (L1), Forex (L1), Oil (L2), Crypto (L3),
            Fear & Greed (L2), WTO Trade (L3), Supply Chain (L3), Gulf FDI (L3) */}
        <PanelStack side="right" collapsed={rightPanelCollapsed}>
          <Panel title={t('panel.marketRadar')} icon={<BarChart3 size={13} />} panelId="market-radar">
            <ErrorBoundary>
              <MarketRadarPanel />
            </ErrorBoundary>
          </Panel>
          <Panel title={t('panel.indices')} icon={<DollarSign size={13} />} panelId="indices">
            <ErrorBoundary>
              <IndicesPanel />
            </ErrorBoundary>
          </Panel>
          <Panel title={t('panel.forex')} icon={<Globe size={13} />} panelId="forex">
            <ErrorBoundary>
              <ForexPanel />
            </ErrorBoundary>
          </Panel>
          <Panel title={t('panel.oilEnergy')} icon={<Droplets size={13} />} panelId="oil-energy">
            <ErrorBoundary>
              <OilEnergyPanel />
            </ErrorBoundary>
          </Panel>
          <Panel title={t('panel.crypto')} icon={<Bitcoin size={13} />} panelId="crypto">
            <ErrorBoundary>
              <CryptoPanel />
            </ErrorBoundary>
          </Panel>
          <Panel title={t('panel.fearGreed')} icon={<TrendingUp size={13} />} panelId="fear-greed">
            <ErrorBoundary>
              <FearGreedPanel />
            </ErrorBoundary>
          </Panel>
          <Panel title={t('panel.wtoTrade')} icon={<Globe size={13} />} panelId="wto-trade">
            <ErrorBoundary>
              <WtoPanel />
            </ErrorBoundary>
          </Panel>
          <Panel title={t('panel.supplyChain')} icon={<Link2 size={13} />} panelId="supply-chain">
            <ErrorBoundary>
              <SupplyChainPanel />
            </ErrorBoundary>
          </Panel>
          <Panel title={t('panel.gulfFdi')} icon={<Building2 size={13} />} panelId="gulf-fdi">
            <ErrorBoundary>
              <GulfFdiPanel />
            </ErrorBoundary>
          </Panel>
        </PanelStack>
      </div>

      <StatusBar />
      <CmdKModal open={cmdKOpen} onClose={() => setCmdKOpen(false)} />
      <SettingsPage open={settingsOpen} onClose={closeSettings} initialTab={settingsInitialTab} />
    </div>
  );
}

export default App;
