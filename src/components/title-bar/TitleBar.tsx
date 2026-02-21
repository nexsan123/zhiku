import { getCurrentWindow } from '@tauri-apps/api/window';
import { Bell, Minus, Square, X } from 'lucide-react';
import './TitleBar.css';

export function TitleBar() {
  const handleMinimize = (): void => {
    void getCurrentWindow().minimize();
  };

  const handleMaximize = (): void => {
    void getCurrentWindow().toggleMaximize();
  };

  const handleClose = (): void => {
    void getCurrentWindow().close();
  };

  return (
    <header className="title-bar">
      <div className="title-bar__drag-region" data-tauri-drag-region>
        <div className="title-bar__logo">智</div>
        <span className="title-bar__title">智库</span>
      </div>

      <div className="title-bar__actions">
        <button className="title-bar__notification-btn" aria-label="Notifications">
          <Bell size={18} />
          <span className="title-bar__notification-badge" aria-label="0 notifications" />
        </button>
      </div>

      <div className="title-bar__controls">
        <button
          className="window-control"
          onClick={handleMinimize}
          aria-label="Minimize"
        >
          <Minus size={10} strokeWidth={1} />
        </button>
        <button
          className="window-control"
          onClick={handleMaximize}
          aria-label="Maximize"
        >
          <Square size={10} strokeWidth={1} />
        </button>
        <button
          className="window-control window-control--close"
          onClick={handleClose}
          aria-label="Close"
        >
          <X size={10} strokeWidth={1} />
        </button>
      </div>
    </header>
  );
}
