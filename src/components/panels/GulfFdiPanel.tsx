import { useTranslation } from 'react-i18next';
import './GulfFdiPanel.css';

interface GulfFdiItem {
  flag: string;
  name: string;
  /** FDI inflow in billion USD */
  fdi: number;
  /** Year-on-year change in percent */
  yoyPct: number;
}

// Static curated FDI data for Gulf Cooperation Council (GCC) states.
// Source: World Bank / UNCTAD estimates 2024. To be replaced by live data in a future phase.
const GULF_FDI: GulfFdiItem[] = [
  { flag: '\uD83C\uDDE6\uD83C\uDDEA', name: 'UAE',    fdi: 23.1, yoyPct:  12.3 },
  { flag: '\uD83C\uDDF8\uD83C\uDDE6', name: '沙特',   fdi: 18.7, yoyPct:  -3.1 },
  { flag: '\uD83C\uDDF6\uD83C\uDDE6', name: '卡塔尔', fdi:  8.2, yoyPct:   5.4 },
  { flag: '\uD83C\uDDF0\uD83C\uDDFC', name: '科威特', fdi:  2.1, yoyPct:   1.2 },
  { flag: '\uD83C\uDDE7\uD83C\uDDED', name: '巴林',   fdi:  1.3, yoyPct:  -1.8 },
  { flag: '\uD83C\uDDF4\uD83C\uDDF2', name: '阿曼',   fdi:  3.4, yoyPct:   7.6 },
];

export function GulfFdiPanel() {
  const { t } = useTranslation();

  return (
    <div className="gulf-fdi">
      <ul className="gulf-fdi__list" aria-label="Gulf FDI by country">
        {GULF_FDI.map((item) => {
          const yoyPositive = item.yoyPct >= 0;
          return (
            <li key={item.name} className="gulf-fdi__row">
              <div className="gulf-fdi__left">
                <span className="gulf-fdi__flag" aria-hidden="true">{item.flag}</span>
                <span className="gulf-fdi__country">{item.name}</span>
              </div>
              <div className="gulf-fdi__right">
                <span className="gulf-fdi__fdi">
                  ${item.fdi.toFixed(1)}<span className="gulf-fdi__unit">{t('gulfFdi.unit')}</span>
                </span>
                <span
                  className={`gulf-fdi__yoy ${yoyPositive ? 'gulf-fdi__yoy--up' : 'gulf-fdi__yoy--down'}`}
                  title={t('gulfFdi.yoy')}
                >
                  {yoyPositive ? '+' : ''}{item.yoyPct.toFixed(1)}%
                </span>
              </div>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
