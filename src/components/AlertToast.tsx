import { useState, useEffect, useRef } from 'react';
import { listenAlertsTriggered } from '@services/tauri-bridge';
import type { Alert } from '@services/tauri-bridge';
import './AlertToast.css';

const MAX_TOASTS = 5;
const AUTO_DISMISS_MS = 8000;

function severityIcon(severity: Alert['severity']): string {
  if (severity === 'critical') return '🔴';
  if (severity === 'warning') return '🟡';
  return '🔵';
}

interface ToastItem extends Alert {
  /** Unique key per display (same id can re-appear if threshold triggers again). */
  displayKey: string;
  fadingOut: boolean;
}

function AlertItem({
  item,
  onClose,
}: {
  item: ToastItem;
  onClose: (displayKey: string) => void;
}) {
  return (
    <div
      className={`alert-toast alert-toast--${item.severity}${item.fadingOut ? ' alert-toast--fade-out' : ''}`}
      role="alert"
      aria-live="assertive"
    >
      <div className="alert-toast__icon" aria-hidden="true">
        {severityIcon(item.severity)}
      </div>
      <div className="alert-toast__content">
        <div className="alert-toast__title">{item.title}</div>
        <div className="alert-toast__detail">{item.detail}</div>
        <div className="alert-toast__meta">
          {item.category} · {item.indicatorValue.toFixed(2)} / {item.threshold.toFixed(2)}
        </div>
      </div>
      <button
        className="alert-toast__close"
        onClick={() => onClose(item.displayKey)}
        aria-label="Dismiss alert"
      >
        ×
      </button>
    </div>
  );
}

export function AlertToast() {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  // Track per-toast dismiss timers so we can clear them on manual close
  const timersRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  const dismiss = (displayKey: string) => {
    // Start fade-out animation
    setToasts((prev) =>
      prev.map((t) => (t.displayKey === displayKey ? { ...t, fadingOut: true } : t)),
    );
    // Remove after animation duration (matches CSS 200ms)
    const removeTimer = setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.displayKey !== displayKey));
      timersRef.current.delete(displayKey);
    }, 250);
    // Store remove timer so it can be cleared if needed
    timersRef.current.set(`remove_${displayKey}`, removeTimer);
  };

  useEffect(() => {
    let cleanup: (() => void) | null = null;
    const unlistenPromise = listenAlertsTriggered((newAlerts) => {
      const now = Date.now();
      const incoming: ToastItem[] = newAlerts.slice(0, MAX_TOASTS).map((a, i) => ({
        ...a,
        displayKey: `${a.id}-${now}-${i}`,
        fadingOut: false,
      }));

      setToasts((prev) => {
        const combined = [...incoming, ...prev].slice(0, MAX_TOASTS);
        return combined;
      });

      // Schedule auto-dismiss for each new toast
      incoming.forEach((item) => {
        const timer = setTimeout(() => {
          dismiss(item.displayKey);
        }, AUTO_DISMISS_MS);
        timersRef.current.set(item.displayKey, timer);
      });
    });

    void unlistenPromise.then((fn) => { cleanup = fn; });
    return () => {
      if (cleanup) { cleanup(); }
      else { void unlistenPromise.then((fn) => fn()); }
      // Clear all pending timers
      timersRef.current.forEach((timer) => clearTimeout(timer));
      timersRef.current.clear();
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (toasts.length === 0) return null;

  return (
    <div className="alert-toast-container" aria-label="Alert notifications">
      {toasts.map((item) => (
        <AlertItem key={item.displayKey} item={item} onClose={dismiss} />
      ))}
    </div>
  );
}
