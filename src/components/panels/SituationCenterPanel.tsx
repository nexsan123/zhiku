import { useTranslation } from 'react-i18next';
import { useAppStore } from '@stores/app-store';
import type { SituationTab } from '@stores/app-store';
import { CycleReasoningPanel } from './CycleReasoningPanel';
import { CreditCyclePanel } from './CreditCyclePanel';
import { IntelBriefPanel } from './IntelBriefPanel';
import { GameMapPanel } from './GameMapPanel';
import './SituationCenterPanel.css';

export function SituationCenterPanel() {
  const { t } = useTranslation();
  const activeTab = useAppStore((s) => s.situationTab);
  const setActiveTab = useAppStore((s) => s.setSituationTab);

  const tabs: { id: SituationTab; label: string }[] = [
    { id: 'cycle', label: t('situation.cycleTab') },
    { id: 'credit', label: t('situation.creditTab') },
    { id: 'intel', label: t('situation.intelTab') },
    { id: 'gameMap', label: t('situation.gameMapTab') },
  ];

  return (
    <div className="situation-center">
      <div className="situation-center__tabs" role="tablist" aria-label={t('panel.situationCenter')}>
        {tabs.map((tab) => (
          <button
            key={tab.id}
            className={`situation-center__tab${activeTab === tab.id ? ' situation-center__tab--active' : ''}`}
            onClick={() => setActiveTab(tab.id)}
            role="tab"
            aria-selected={activeTab === tab.id}
            aria-controls={`situation-tab-${tab.id}`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      <div
        className="situation-center__content"
        id={`situation-tab-${activeTab}`}
        role="tabpanel"
      >
        {activeTab === 'cycle' && <CycleReasoningPanel />}
        {activeTab === 'credit' && <CreditCyclePanel />}
        {activeTab === 'intel' && <IntelBriefPanel />}
        {activeTab === 'gameMap' && <GameMapPanel />}
      </div>
    </div>
  );
}
