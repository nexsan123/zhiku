import { getCurrentWindow } from '@tauri-apps/api/window';
import { useTranslation } from 'react-i18next';
import { Search, Radio, Zap, Bell, Minus, Square, X, PanelLeft, PanelRight } from 'lucide-react';
import { useAppStore } from '@stores/app-store';
import './TitleBar.css';

export function TitleBar() {
  const { t, i18n } = useTranslation();
  const toggleLeftPanel = useAppStore((s) => s.toggleLeftPanel);
  const toggleRightPanel = useAppStore((s) => s.toggleRightPanel);
  const notificationCount = useAppStore((s) => s.notificationCount);
  const intelCount = useAppStore((s) => s.intelCount);
  const apiStatus = useAppStore((s) => s.apiStatus);

  const sourceValues = Object.values(apiStatus);
  const onlineSourceCount = sourceValues.filter((s) => s.status === 'online').length;
  const totalSourceCount = sourceValues.length;

  const handleMinimize = (): void => {
    void getCurrentWindow().minimize();
  };

  const handleMaximize = (): void => {
    void getCurrentWindow().toggleMaximize();
  };

  const handleClose = (): void => {
    void getCurrentWindow().close();
  };

  const handleLanguageToggle = (): void => {
    void i18n.changeLanguage(i18n.language === 'zh-CN' ? 'en' : 'zh-CN');
  };

  return (
    <header className="title-bar">
      {/* Left: Logo + App name + variant badge + left panel toggle */}
      <div className="title-bar__left" data-tauri-drag-region>
        <div className="title-bar__logo" aria-hidden="true">智</div>
        <span className="title-bar__appname">库</span>
        <span className="title-bar__variant">{t('app.finance')}</span>
        <button
          className="title-bar__panel-btn"
          onClick={toggleLeftPanel}
          aria-label={t('titleBar.toggleLeftPanel')}
        >
          <PanelLeft size={14} />
        </button>
      </div>

      {/* Center: Search + Sources + Intel */}
      <div className="title-bar__center" data-tauri-drag-region>
        <button className="title-bar__action-btn" aria-label={`${t('titleBar.search')} (${t('titleBar.searchShortcut')})`}>
          <Search size={13} />
          <span className="title-bar__action-label">{t('titleBar.search')}</span>
          <kbd className="title-bar__kbd">{t('titleBar.searchShortcut')}</kbd>
        </button>
        <button className="title-bar__action-btn" aria-label={t('titleBar.sources')}>
          <Radio size={13} />
          <span className="title-bar__action-label">{t('titleBar.sources')}</span>
          <span className="title-bar__action-count">{onlineSourceCount}/{totalSourceCount}</span>
        </button>
        <button className="title-bar__action-btn" aria-label={t('titleBar.intel')}>
          <Zap size={13} />
          <span className="title-bar__action-label">{t('titleBar.intel')}</span>
          <span className="title-bar__badge">{intelCount}</span>
        </button>
      </div>

      {/* Right: language toggle + right panel toggle + notification bell + window controls */}
      <div className="title-bar__right">
        <button
          className="title-bar__action-btn"
          onClick={handleLanguageToggle}
          aria-label={t('language.switch')}
        >
          <span className="title-bar__action-label">
            {i18n.language === 'zh-CN' ? 'EN' : '中'}
          </span>
        </button>
        <button
          className="title-bar__panel-btn"
          onClick={toggleRightPanel}
          aria-label={t('titleBar.toggleRightPanel')}
        >
          <PanelRight size={14} />
        </button>
        <button
          className="title-bar__notification-btn"
          aria-label={`Notifications (${notificationCount})`}
        >
          <Bell size={16} />
          {notificationCount > 0 && (
            <span className="title-bar__notification-badge" aria-hidden="true">
              {notificationCount}
            </span>
          )}
        </button>

        <div className="title-bar__controls">
          <button className="window-control" onClick={handleMinimize} aria-label={t('titleBar.minimize')}>
            <Minus size={10} strokeWidth={1} />
          </button>
          <button className="window-control" onClick={handleMaximize} aria-label={t('titleBar.maximize')}>
            <Square size={10} strokeWidth={1} />
          </button>
          <button
            className="window-control window-control--close"
            onClick={handleClose}
            aria-label={t('titleBar.close')}
          >
            <X size={10} strokeWidth={1} />
          </button>
        </div>
      </div>
    </header>
  );
}
