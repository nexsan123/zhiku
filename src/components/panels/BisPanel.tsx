import { useTranslation } from 'react-i18next';
import './BisPanel.css';

type RateDir = 'up' | 'down' | 'hold';

interface BisRateItem {
  flag: string;
  nameKey: string;
  nameEnKey: string;
  rate: number;
  dir: RateDir;
}

// Static curated data — current known policy rates as of 2026-03.
// Will be replaced by real BIS API data in a future phase.
const BIS_RATES: BisRateItem[] = [
  { flag: '\uD83C\uDDFA\uD83C\uDDF8', nameKey: 'bis.fed',  nameEnKey: 'Fed',  rate: 5.25, dir: 'hold' },
  { flag: '\uD83C\uDDEA\uD83C\uDDFA', nameKey: 'bis.ecb',  nameEnKey: 'ECB',  rate: 4.50, dir: 'hold' },
  { flag: '\uD83C\uDDEF\uD83C\uDDF5', nameKey: 'bis.boj',  nameEnKey: 'BoJ',  rate: 0.10, dir: 'up'   },
  { flag: '\uD83C\uDDEC\uD83C\uDDE7', nameKey: 'bis.boe',  nameEnKey: 'BoE',  rate: 5.25, dir: 'hold' },
  { flag: '\uD83C\uDDE8\uD83C\uDDF3', nameKey: 'bis.pboc', nameEnKey: 'PBoC', rate: 3.45, dir: 'down' },
];

function dirArrow(dir: RateDir): string {
  if (dir === 'up')   return '↑';
  if (dir === 'down') return '↓';
  return '—';
}

export function BisPanel() {
  const { t } = useTranslation();

  return (
    <div className="bis">
      <ul className="bis__list" aria-label="Central bank policy rates">
        {BIS_RATES.map((item) => {
          const arrow = dirArrow(item.dir);
          return (
            <li key={item.nameEnKey} className="bis__row">
              <div className="bis__left">
                <span className="bis__flag" aria-hidden="true">{item.flag}</span>
                <div className="bis__names">
                  <span className="bis__name-en">{item.nameEnKey}</span>
                  <span className="bis__label">{t(item.nameKey)}</span>
                </div>
              </div>
              <div className="bis__right">
                <span className="bis__rate">{item.rate.toFixed(2)}%</span>
                <span
                  className={`bis__dir bis__dir--${item.dir}`}
                  title={t(`bis.${item.dir}`)}
                  aria-label={t(`bis.${item.dir}`)}
                >
                  {arrow}
                </span>
              </div>
            </li>
          );
        })}
      </ul>

      <div className="bis__footer">
        <span className="bis__footer-note">{t('bis.staticNote')}</span>
      </div>
    </div>
  );
}
