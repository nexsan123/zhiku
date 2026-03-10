import { useTranslation } from 'react-i18next';
import './WtoPanel.css';

// Static curated trade data — to be replaced by real WTO API data in a future phase.
const TRADE_EVENT_KEYS = [
  'wto.event1',
  'wto.event2',
  'wto.event3',
  'wto.event4',
] as const;

export function WtoPanel() {
  const { t } = useTranslation();

  return (
    <div className="wto">
      {/* Global trade volume card */}
      <div className="wto__overview-card">
        <span className="wto__overview-label">{t('wto.tradeVolume')}</span>
        <span className="wto__overview-value">{t('wto.tradeVolumeValue')}</span>
      </div>

      {/* Trade friction events */}
      <div className="wto__section">
        <h4 className="wto__section-title">{t('wto.tradeEvents')}</h4>
        <ul className="wto__events" aria-label="Trade friction events">
          {TRADE_EVENT_KEYS.map((key) => (
            <li key={key} className="wto__event-item">
              <span className="wto__event-bullet" aria-hidden="true">▸</span>
              <span className="wto__event-text">{t(key)}</span>
            </li>
          ))}
        </ul>
      </div>

      <div className="wto__footer">
        <span className="wto__footer-note">{t('wto.staticNote')}</span>
      </div>
    </div>
  );
}
