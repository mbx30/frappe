import React from 'react';

const TONES = {
  neutral: { accent: 'var(--text-primary)', icon: null },
  success: { accent: 'var(--success)', icon: 'M20 6 9 17l-5-5' },
  warning: { accent: 'var(--warning)', icon: 'M12 9v4M12 17h.01M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z' },
  danger:  { accent: 'var(--danger)', icon: 'M12 9v4M12 17h.01M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z' },
  info:    { accent: 'var(--info)', icon: 'M12 16v-4M12 8h.01' },
};

/**
 * Toast — a single transient notification card. Compose several inside
 * `ToastViewport`. Self-dismisses after `duration` ms (0 = sticky).
 */
export function Toast({ title, message, tone = 'neutral', onClose, action, duration = 4000 }) {
  const t = TONES[tone] || TONES.neutral;
  React.useEffect(() => {
    if (!duration) return;
    const id = setTimeout(() => onClose && onClose(), duration);
    return () => clearTimeout(id);
  }, [duration, onClose]);

  return (
    <div role="status" style={{
      display: 'flex', alignItems: 'flex-start', gap: '11px', width: 340, maxWidth: '90vw',
      padding: '12px 14px', background: 'var(--surface-raised)',
      border: '1px solid var(--border-default)', borderLeft: `3px solid ${t.accent}`,
      borderRadius: 'var(--radius-md)', boxShadow: 'var(--shadow-lg)',
      animation: 'frappe-toast var(--duration-base) var(--ease-out)',
    }}>
      <style>{'@keyframes frappe-toast{from{opacity:0;transform:translateX(12px)}to{opacity:1;transform:none}}'}</style>
      {t.icon && (
        <span style={{ flex: 'none', color: t.accent, marginTop: '1px' }}>
          <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.1" strokeLinecap="round" strokeLinejoin="round"><path d={t.icon} /></svg>
        </span>
      )}
      <div style={{ flex: 1, minWidth: 0 }}>
        {title && <div style={{ font: 'var(--font-body-strong)', color: 'var(--text-primary)' }}>{title}</div>}
        {message && <div style={{ font: 'var(--font-caption)', color: 'var(--text-secondary)', marginTop: '2px' }}>{message}</div>}
        {action && <div style={{ marginTop: '8px' }}>{action}</div>}
      </div>
      {onClose && (
        <button type="button" aria-label="Dismiss" onClick={onClose} style={{
          flex: 'none', border: 'none', background: 'transparent', cursor: 'pointer',
          color: 'var(--text-tertiary)', padding: '2px', marginTop: '-2px',
        }}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
      )}
    </div>
  );
}

/**
 * ToastViewport — fixed stacking container. Place once near the app root.
 */
export function ToastViewport({ children, placement = 'bottom-right' }) {
  const pos = {
    'bottom-right': { bottom: 20, right: 20, alignItems: 'flex-end' },
    'bottom-left': { bottom: 20, left: 20, alignItems: 'flex-start' },
    'top-right': { top: 20, right: 20, alignItems: 'flex-end' },
    'top-center': { top: 20, left: '50%', transform: 'translateX(-50%)', alignItems: 'center' },
  }[placement] || {};
  return (
    <div style={{
      position: 'fixed', zIndex: 'var(--z-toast)', display: 'flex', flexDirection: 'column',
      gap: '10px', ...pos,
    }}>{children}</div>
  );
}
