import React from 'react';

/**
 * Frappe Tooltip — hover/focus label for icon buttons and truncated text.
 * Lightweight, no portal; positions relative to a wrapped trigger.
 */
export function Tooltip({ label, children, side = 'top', delay = 250 }) {
  const [open, setOpen] = React.useState(false);
  const timer = React.useRef(null);

  const show = () => { timer.current = setTimeout(() => setOpen(true), delay); };
  const hide = () => { clearTimeout(timer.current); setOpen(false); };

  const pos = {
    top:    { bottom: '100%', left: '50%', transform: 'translateX(-50%)', marginBottom: '7px' },
    bottom: { top: '100%', left: '50%', transform: 'translateX(-50%)', marginTop: '7px' },
    left:   { right: '100%', top: '50%', transform: 'translateY(-50%)', marginRight: '7px' },
    right:  { left: '100%', top: '50%', transform: 'translateY(-50%)', marginLeft: '7px' },
  }[side] || {};

  return (
    <span
      style={{ position: 'relative', display: 'inline-flex' }}
      onMouseEnter={show}
      onMouseLeave={hide}
      onFocus={show}
      onBlur={hide}
    >
      {children}
      {open && (
        <span
          role="tooltip"
          style={{
            position: 'absolute',
            zIndex: 'var(--z-tooltip)',
            ...pos,
            background: 'var(--neutral-900)',
            color: '#fff',
            font: 'var(--font-caption)',
            fontWeight: 'var(--weight-medium)',
            padding: '5px 8px',
            borderRadius: 'var(--radius-sm)',
            boxShadow: 'var(--shadow-md)',
            whiteSpace: 'nowrap',
            pointerEvents: 'none',
            letterSpacing: 'var(--tracking-tight)',
            animation: 'frappe-tip-in var(--duration-fast) var(--ease-out)',
          }}
        >
          <style>{'@keyframes frappe-tip-in{from{opacity:0;transform:' + (pos.transform || '') + ' scale(0.96)}to{opacity:1}}'}</style>
          {label}
        </span>
      )}
    </span>
  );
}
