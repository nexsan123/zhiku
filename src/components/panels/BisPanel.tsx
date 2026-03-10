import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw } from 'lucide-react';
import { getMacroData } from '@services/tauri-bridge';
import type { MacroDataItem } from '@services/tauri-bridge';
import './BisPanel.css';

type LoadState = 'loading' | 'loaded' | 'error';
type RateDir = 'up' | 'down' | 'hold';

// ---- Config for 13 central banks tracked by BIS ----
interface BisConfig {
  /** Country/region code matching BIS_CBPOL_<code> in macro_data. */
  code: string;
  flag: string;
  /** Short abbreviation shown in the row. */
  abbr: string;
  /** i18n key for the long name (maps to existing bis.* keys). */
  nameKey: string;
}

const BIS_CONFIGS: BisConfig[] = [
  { code: 'US', flag: '\uD83C\uDDFA\uD83C\uDDF8', abbr: 'Fed',   nameKey: 'bis.fed'   },
  { code: 'EU', flag: '\uD83C\uDDEA\uD83C\uDDFA', abbr: 'ECB',   nameKey: 'bis.ecb'   },
  { code: 'JP', flag: '\uD83C\uDDEF\uD83C\uDDF5', abbr: 'BoJ',   nameKey: 'bis.boj'   },
  { code: 'UK', flag: '\uD83C\uDDEC\uD83C\uDDE7', abbr: 'BoE',   nameKey: 'bis.boe'   },
  { code: 'CN', flag: '\uD83C\uDDE8\uD83C\uDDF3', abbr: 'PBoC',  nameKey: 'bis.pboc'  },
  { code: 'IN', flag: '\uD83C\uDDEE\uD83C\uDDF3', abbr: 'RBI',   nameKey: 'bis.rbi'   },
  { code: 'CA', flag: '\uD83C\uDDE8\uD83C\uDDE6', abbr: 'BoC',   nameKey: 'bis.boc'   },
  { code: 'AU', flag: '\uD83C\uDDE6\uD83C\uDDFA', abbr: 'RBA',   nameKey: 'bis.rba'   },
  { code: 'CH', flag: '\uD83C\uDDE8\uD83C\uDDED', abbr: 'SNB',   nameKey: 'bis.snb'   },
  { code: 'KR', flag: '\uD83C\uDDF0\uD83C\uDDF7', abbr: 'BoK',   nameKey: 'bis.bok'   },
  { code: 'BR', flag: '\uD83C\uDDE7\uD83C\uDDF7', abbr: 'BCB',   nameKey: 'bis.bcb'   },
  { code: 'SA', flag: '\uD83C\uDDF8\uD83C\uDDE6', abbr: 'SAMA',  nameKey: 'bis.sama'  },
  { code: 'AE', flag: '\uD83C\uDDE6\uD83C\uDDEA', abbr: 'CBUAE', nameKey: 'bis.cbuae' },
];

// ---- Static fallback data shown when backend has no BIS rows yet ----
interface StaticBisRow {
  code: string;
  rate: number;
  dir: RateDir;
}

const STATIC_BIS_FALLBACK: StaticBisRow[] = [
  { code: 'US', rate: 5.25, dir: 'hold' },
  { code: 'EU', rate: 4.50, dir: 'hold' },
  { code: 'JP', rate: 0.10, dir: 'up'   },
  { code: 'UK', rate: 5.25, dir: 'hold' },
  { code: 'CN', rate: 3.45, dir: 'down' },
];

function dirArrow(dir: RateDir): string {
  if (dir === 'up')   return '↑';
  if (dir === 'down') return '↓';
  return '—';
}

/** Infer direction from macro_data by looking at the value relative to a prior
 *  period.  Since we only have single-period snapshots, we default to 'hold'.
 *  Future: compare two consecutive rows to derive real direction. */
function inferDir(_value: number): RateDir {
  return 'hold';
}

export function BisPanel() {
  const { t } = useTranslation();
  const [bisRows, setBisRows] = useState<MacroDataItem[]>([]);
  const [loadState, setLoadState] = useState<LoadState>('loading');

  const load = useCallback(async () => {
    setLoadState('loading');
    try {
      const data = await getMacroData();
      const filtered = data.filter((d) => d.indicator.startsWith('BIS_CBPOL_'));
      setBisRows(filtered);
      setLoadState('loaded');
    } catch {
      setLoadState('error');
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  // ---- Loading state ----
  if (loadState === 'loading') {
    return (
      <div className="bis__state">
        <RefreshCw size={14} className="bis__spinner" />
        <span className="bis__state-text">{t('bis.loading')}</span>
      </div>
    );
  }

  // ---- Error state ----
  if (loadState === 'error') {
    return (
      <div className="bis__state bis__state--error">
        <p className="bis__state-text">{t('bis.failed')}</p>
        <button className="bis__retry-btn" onClick={() => void load()}>
          {t('state.retry')}
        </button>
      </div>
    );
  }

  // ---- Loaded: determine whether we have live data or need static fallback ----
  const hasLiveData = bisRows.length > 0;

  // Build a lookup map from live data  e.g.  "US" -> MacroDataItem
  const liveMap = new Map(
    bisRows.map((row) => [row.indicator.replace('BIS_CBPOL_', ''), row])
  );

  // Determine which configs to render:
  // - Live data: all 13 configs, showing '--' for missing codes
  // - No live data: only the 5 static fallback entries
  const configsToRender = hasLiveData
    ? BIS_CONFIGS
    : BIS_CONFIGS.filter((c) => STATIC_BIS_FALLBACK.some((s) => s.code === c.code));

  return (
    <div className="bis">
      {!hasLiveData && (
        <div className="bis__notice">
          {t('bis.noData')}
        </div>
      )}

      <ul className="bis__list" aria-label="Central bank policy rates">
        {configsToRender.map((config) => {
          const liveRow = liveMap.get(config.code);
          const staticRow = STATIC_BIS_FALLBACK.find((s) => s.code === config.code);

          const rate = liveRow?.value ?? staticRow?.rate;
          const dir: RateDir = liveRow ? inferDir(liveRow.value) : (staticRow?.dir ?? 'hold');
          const arrow = dirArrow(dir);
          const rateDisplay = rate !== undefined ? `${rate.toFixed(2)}%` : '--';

          return (
            <li key={config.code} className="bis__row">
              <div className="bis__left">
                <span className="bis__flag" aria-hidden="true">{config.flag}</span>
                <div className="bis__names">
                  <span className="bis__name-en">{config.abbr}</span>
                  <span className="bis__label">{t(config.nameKey)}</span>
                </div>
              </div>
              <div className="bis__right">
                <span className="bis__rate">{rateDisplay}</span>
                <span
                  className={`bis__dir bis__dir--${dir}`}
                  title={t(`bis.${dir}`)}
                  aria-label={t(`bis.${dir}`)}
                >
                  {arrow}
                </span>
              </div>
            </li>
          );
        })}
      </ul>

      <div className="bis__footer">
        <span className="bis__footer-note">
          {hasLiveData ? t('bis.liveNote') : t('bis.staticNote')}
        </span>
      </div>
    </div>
  );
}
