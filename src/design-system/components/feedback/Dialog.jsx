import React from 'react';

/**
 * Dialog — centered modal with scrim, header, body and footer actions.
 * Closes on scrim click and Escape.
 */
export function Dialog({
  open, onClose, title, description, children, footer, width = 480, closeOnScrim = true, ...rest
}) {
  React.useEffect(() => {
    if (!open) return;
    const onKey = (e) => { if (e.key === 'Escape') onClose && onClose(); };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      onMouseDown={(e) => { if (closeOnScrim && e.target === e.currentTarget) onClose && onClose(); }}
      style={{
        position: 'fixed', inset: 0, zIndex: 'var(--z-modal)',
        background: 'var(--surface-overlay)', backdropFilter: 'blur(2px)',
        display: 'flex', alignItems: 'center', justifyContent: 'center', padding: '24px',
        animation: 'frappe-fade var(--duration-fast) var(--ease-standard)',
      }}
    >
      <style>{'@keyframes frappe-fade{from{opacity:0}to{opacity:1}}@keyframes frappe-pop{from{opacity:0;transform:translateY(8px) scale(0.98)}to{opacity:1;transform:none}}'}</style>
      <div role="dialog" aria-modal="true" aria-label={typeof title === 'string' ? title : undefined}
        style={{
          width, maxWidth: '100%', maxHeight: '90vh', display: 'flex', flexDirection: 'column',
          background: 'var(--surface-raised)', border: '1px solid var(--border-default)',
          borderRadius: 'var(--radius-xl)', boxShadow: 'var(--shadow-lg)', overflow: 'hidden',
          animation: 'frappe-pop var(--duration-base) var(--ease-out)',
        }} {...rest}>
        {(title || onClose) && (
          <div style={{
            display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: '16px',
            padding: '18px 20px 14px',
          }}>
            <div style={{ minWidth: 0 }}>
              {title && <div style={{ font: 'var(--font-h3)', color: 'var(--text-primary)' }}>{title}</div>}
              {description && <div style={{ font: 'var(--font-body)', color: 'var(--text-secondary)', marginTop: '4px' }}>{description}</div>}
            </div>
            {onClose && (
              <button type="button" aria-label="Close" onClick={onClose} style={{
                display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
                width: 30, height: 30, borderRadius: 'var(--radius-md)', border: 'none', flex: 'none',
                background: 'transparent', color: 'var(--text-tertiary)', cursor: 'pointer',
              }}>
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
              </button>
            )}
          </div>
        )}
        {children != null && (
          <div style={{ padding: '0 20px 16px', overflowY: 'auto', font: 'var(--font-body)', color: 'var(--text-secondary)' }}>{children}</div>
        )}
        {footer && (
          <div style={{
            display: 'flex', justifyContent: 'flex-end', gap: '8px',
            padding: '14px 20px', borderTop: '1px solid var(--border-subtle)', background: 'var(--bg-subtle)',
          }}>{footer}</div>
        )}
      </div>
    </div>
  );
}
