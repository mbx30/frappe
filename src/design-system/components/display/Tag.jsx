import React from 'react';

/**
 * Tag — small removable chip for filters, attributes, and selections.
 * Quieter than Badge; squared corners; optional dismiss button.
 */
export function Tag({ children, onRemove, color, icon, style, ...rest }) {
  const [hover, setHover] = React.useState(false);
  return (
    <span style={{
      display: 'inline-flex', alignItems: 'center', gap: '6px',
      padding: '3px 6px 3px 8px', borderRadius: 'var(--radius-sm)',
      background: 'var(--surface-active)', color: 'var(--text-secondary)',
      border: '1px solid var(--border-default)',
      font: 'var(--font-sans)', fontSize: 'var(--text-xs)', fontWeight: 'var(--weight-medium)',
      lineHeight: 1.4, whiteSpace: 'nowrap', ...style,
    }} {...rest}>
      {color && <span style={{ width: 7, height: 7, borderRadius: '2px', background: color, flex: 'none' }} />}
      {icon}
      <span style={{ paddingRight: onRemove ? 0 : '2px' }}>{children}</span>
      {onRemove && (
        <button type="button" aria-label="Remove" onClick={onRemove}
          onMouseEnter={() => setHover(true)} onMouseLeave={() => setHover(false)}
          style={{
            display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
            width: 15, height: 15, borderRadius: 'var(--radius-xs)', border: 'none', padding: 0,
            background: hover ? 'var(--surface-overlay)' : 'transparent',
            color: hover ? 'var(--text-primary)' : 'var(--text-tertiary)', cursor: 'pointer',
          }}>
          <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
      )}
    </span>
  );
}
