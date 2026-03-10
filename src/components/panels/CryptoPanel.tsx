import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw } from 'lucide-react';
import { getMarketData, getMacroData, isTauri } from '@services/tauri-bridge';
import type { MarketDataItem } from '@services/tauri-bridge';
import type { MockStablecoin } from '@utils/mocks/panel-data';
import { MOCK_STABLECOINS } from '@utils/mocks/panel-data';
import './CryptoPanel.css';

type LoadState = 'loading' | 'loaded' | 'error';

// Known stablecoin base symbols (without "-USD" suffix)
const STABLECOIN_SYMBOLS = new Set(['USDT', 'USDC', 'DAI', 'BUSD', 'FRAX', 'TUSD']);

type PegStatus = 'on-peg' | 'slight-depeg' | 'depegged';

// Maps peg status to i18n key (replaces hardcoded PEG_LABELS constant)
const PEG_LABEL_KEY: Record<PegStatus, string> = {
  'on-peg': 'market.onPeg',
  'slight-depeg': 'market.slightDepeg',
  depegged: 'market.depegged',
};

/** Derive peg status from price (stablecoins should be near $1.00) */
function derivePeg(price: number): PegStatus {
  if (price >= 0.999 && price <= 1.001) return 'on-peg';
  if (price >= 0.995 && price <= 1.005) return 'slight-depeg';
  return 'depegged';
}

interface StablecoinDisplay {
  symbol: string;
  peg: PegStatus;
}

/** BTC network health metrics derived from macro_data BTC_* indicators. */
interface BtcNetworkData {
  hashrate: number | null;
  feeMedium: number | null;
  feeFast: number | null;
  difficultyProgress: number | null;
  difficultyChange: number | null;
}

/** Format hashrate: raw value from backend is in EH/s (exahashes/second). */
function formatHashrate(v: number | null): string {
  if (v === null || isNaN(v)) return '--';
  if (v >= 1) return `${v.toFixed(1)} EH/s`;
  // If value < 1, assume it's stored in TH/s scale
  return `${(v * 1000).toFixed(0)} TH/s`;
}

function formatFee(v: number | null): string {
  if (v === null || isNaN(v)) return '--';
  return `${Math.round(v)} sat/vB`;
}

function formatDiffProgress(v: number | null): string {
  if (v === null || isNaN(v)) return '--';
  return `${v.toFixed(1)}%`;
}

function formatDiffChange(v: number | null): string {
  if (v === null || isNaN(v)) return '--';
  const sign = v >= 0 ? '+' : '';
  return `${sign}${v.toFixed(2)}%`;
}

export function CryptoPanel() {
  const { t } = useTranslation();
  const [assets, setAssets] = useState<MarketDataItem[]>([]);
  const [stables, setStables] = useState<StablecoinDisplay[]>([]);
  const [loadState, setLoadState] = useState<LoadState>('loading');
  const [btcNetwork, setBtcNetwork] = useState<BtcNetworkData | null>(null);

  const load = useCallback(async () => {
    setLoadState('loading');
    try {
      const data = await getMarketData();

      // Major crypto: ends with "-USD" but NOT a known stablecoin
      const crypto = data.filter((d) => {
        if (!d.symbol.endsWith('-USD')) return false;
        const base = d.symbol.replace('-USD', '');
        return !STABLECOIN_SYMBOLS.has(base);
      });

      // Stablecoin peg status — derive from price in Tauri env; use mock in dev env
      const stablecoinItems = data.filter((d) => {
        if (!d.symbol.endsWith('-USD')) return false;
        const base = d.symbol.replace('-USD', '');
        return STABLECOIN_SYMBOLS.has(base);
      });

      if (isTauri() || stablecoinItems.length > 0) {
        const stableDisplay: StablecoinDisplay[] = stablecoinItems.map((item) => ({
          symbol: item.symbol.replace('-USD', ''),
          peg: derivePeg(item.price),
        }));
        setStables(stableDisplay);
      } else {
        // Dev env fallback: use mock stablecoin data if not present in market data
        setStables(
          (MOCK_STABLECOINS as MockStablecoin[]).map((s) => ({ symbol: s.symbol, peg: s.peg }))
        );
      }

      setAssets(crypto);
      setLoadState('loaded');
    } catch {
      setLoadState('error');
    }

    // BTC network health — independent fetch, silently degrades on failure
    try {
      const macro = await getMacroData();
      const find = (key: string) => macro.find((d) => d.indicator === key)?.value ?? null;
      setBtcNetwork({
        hashrate: find('BTC_HASHRATE'),
        feeMedium: find('BTC_FEE_MEDIUM'),
        feeFast: find('BTC_FEE_FAST'),
        difficultyProgress: find('BTC_DIFFICULTY_PROGRESS'),
        difficultyChange: find('BTC_DIFFICULTY_CHANGE'),
      });
    } catch {
      setBtcNetwork(null);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  if (loadState === 'loading') {
    return (
      <div className="crypto-panel__state">
        <RefreshCw size={14} className="crypto-panel__spinner" />
        <span className="crypto-panel__state-text">{t('state.loadingCrypto')}</span>
      </div>
    );
  }

  if (loadState === 'error') {
    return (
      <div className="crypto-panel__state crypto-panel__state--error">
        <p className="crypto-panel__state-text">{t('state.failedCrypto')}</p>
        <button className="crypto-panel__retry-btn" onClick={() => void load()}>
          {t('state.retry')}
        </button>
      </div>
    );
  }

  if (assets.length === 0) {
    return (
      <div className="crypto-panel__state">
        <p className="crypto-panel__state-text">{t('state.noCrypto')}</p>
      </div>
    );
  }

  return (
    <div className="crypto-panel">
      {/* Major crypto assets */}
      <ul className="crypto-panel__assets" aria-label="Cryptocurrency prices">
        {assets.map((asset, idx) => {
          const pct = asset.changePct ?? 0;
          const displaySymbol = asset.symbol.replace('-USD', '');
          return (
            <li
              key={asset.symbol}
              className={`crypto-panel__asset-row ${
                idx < assets.length - 1 ? 'crypto-panel__asset-row--divider' : ''
              }`}
            >
              <div className="crypto-panel__asset-left">
                <span className="crypto-panel__asset-name">{displaySymbol}</span>
                <span className="crypto-panel__asset-symbol">{asset.symbol}</span>
              </div>
              <div className="crypto-panel__asset-right">
                <span className="crypto-panel__asset-price">
                  ${asset.price.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                </span>
                <span
                  className={`crypto-panel__asset-change ${
                    pct >= 0
                      ? 'crypto-panel__asset-change--up'
                      : 'crypto-panel__asset-change--down'
                  }`}
                >
                  {asset.changePct !== null
                    ? `${pct >= 0 ? '+' : ''}${pct.toFixed(2)}%`
                    : '--'}
                </span>
              </div>
            </li>
          );
        })}
      </ul>

      {/* Stablecoin peg status */}
      {stables.length > 0 && (
        <>
          <div className="crypto-panel__stables-header" aria-label="Stablecoin peg status">
            {t('market.stablecoins')}
          </div>
          <ul className="crypto-panel__stables">
            {stables.map((s) => (
              <li key={s.symbol} className="crypto-panel__stable-row">
                <span className="crypto-panel__stable-symbol">{s.symbol}</span>
                <span className={`crypto-panel__stable-status crypto-panel__stable-status--${s.peg}`}>
                  <span className="crypto-panel__stable-dot" aria-hidden="true" />
                  {t(PEG_LABEL_KEY[s.peg])}
                </span>
              </li>
            ))}
          </ul>
        </>
      )}

      {/* BTC Network Health — shown only when macro_data has BTC_* entries */}
      {btcNetwork && (
        <>
          <div className="crypto-panel__btc-header">
            {t('btcNetwork.title')}
          </div>
          <ul className="crypto-panel__btc-list" aria-label="BTC network health">
            <li className="crypto-panel__btc-row">
              <span className="crypto-panel__btc-label">{t('btcNetwork.hashrate')}</span>
              <span className="crypto-panel__btc-value">{formatHashrate(btcNetwork.hashrate)}</span>
            </li>
            <li className="crypto-panel__btc-row">
              <span className="crypto-panel__btc-label">{t('btcNetwork.feeMedium')}</span>
              <span className="crypto-panel__btc-value">{formatFee(btcNetwork.feeMedium)}</span>
            </li>
            <li className="crypto-panel__btc-row">
              <span className="crypto-panel__btc-label">{t('btcNetwork.feeFast')}</span>
              <span className="crypto-panel__btc-value">{formatFee(btcNetwork.feeFast)}</span>
            </li>
            <li className="crypto-panel__btc-row">
              <span className="crypto-panel__btc-label">{t('btcNetwork.difficultyProgress')}</span>
              <span className="crypto-panel__btc-value">{formatDiffProgress(btcNetwork.difficultyProgress)}</span>
            </li>
            <li className="crypto-panel__btc-row">
              <span className="crypto-panel__btc-label">{t('btcNetwork.difficultyChange')}</span>
              <span
                className={`crypto-panel__btc-value ${
                  btcNetwork.difficultyChange !== null && btcNetwork.difficultyChange >= 0
                    ? 'crypto-panel__btc-value--up'
                    : 'crypto-panel__btc-value--down'
                }`}
              >
                {formatDiffChange(btcNetwork.difficultyChange)}
              </span>
            </li>
          </ul>
        </>
      )}
    </div>
  );
}
