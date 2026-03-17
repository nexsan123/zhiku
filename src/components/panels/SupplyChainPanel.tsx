import { useTranslation } from 'react-i18next';
import './SupplyChainPanel.css';

type HealthStatus = 'green' | 'yellow' | 'red';

interface SupplyChainIndicator {
  nameKey: string;
  status: HealthStatus;
}

interface SupplyChainEvent {
  textKey: string;
}

// Static curated data — to be replaced by real data in a future phase.
const SC_INDICATORS: SupplyChainIndicator[] = [
  { nameKey: 'supplyChain.indicator1', status: 'yellow' },  // 全球航运延误
  { nameKey: 'supplyChain.indicator2', status: 'green'  },  // 半导体库存
  { nameKey: 'supplyChain.indicator3', status: 'green'  },  // 能源供应稳定性
];

const SC_EVENTS: SupplyChainEvent[] = [
  { textKey: 'supplyChain.event1' },
  { textKey: 'supplyChain.event2' },
  { textKey: 'supplyChain.event3' },
  { textKey: 'supplyChain.event4' },
];

function statusLabel(status: HealthStatus, t: (key: string) => string): string {
  if (status === 'green')  return t('supplyChain.statusNormal');
  if (status === 'yellow') return t('supplyChain.statusCaution');
  return t('supplyChain.statusAlert');
}

export function SupplyChainPanel() {
  const { t } = useTranslation();

  return (
    <div className="supply-chain">
      {/* Health indicators */}
      <div className="supply-chain__section">
        <h4 className="supply-chain__section-title">{t('supplyChain.title')}</h4>
        <ul className="supply-chain__indicators" aria-label="Supply chain health indicators">
          {SC_INDICATORS.map((item) => (
            <li key={item.nameKey} className="supply-chain__indicator-row">
              <div className="supply-chain__indicator-left">
                <span
                  className={`supply-chain__dot supply-chain__dot--${item.status}`}
                  aria-hidden="true"
                />
                <span className="supply-chain__indicator-name">{t(item.nameKey)}</span>
              </div>
              <span className={`supply-chain__status-label supply-chain__status-label--${item.status}`}>
                {statusLabel(item.status, t)}
              </span>
            </li>
          ))}
        </ul>
      </div>

      {/* Key events */}
      <div className="supply-chain__section">
        <h4 className="supply-chain__section-title">{t('supplyChain.events')}</h4>
        <ul className="supply-chain__events" aria-label="Supply chain key events">
          {SC_EVENTS.map((ev) => (
            <li key={ev.textKey} className="supply-chain__event-item">
              <span className="supply-chain__event-bullet" aria-hidden="true">▸</span>
              <span className="supply-chain__event-text">{t(ev.textKey)}</span>
            </li>
          ))}
        </ul>
      </div>

      <div className="supply-chain__footer">
        <span className="supply-chain__footer-note">{t('supplyChain.staticNote')}</span>
      </div>
    </div>
  );
}
