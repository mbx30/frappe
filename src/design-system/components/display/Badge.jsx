import React from 'react';

const TONES = {
  neutral: { bg: 'var(--surface-active)', fg: 'var(--text-secondary)', dot: 'var(--text-tertiary)' },
  brand:   { bg: 'var(--brand-subtle)', fg: 'var(--brand-text)', dot: 'var(--brand)' },
  success: { bg: 'var(--success-subtle)', fg: 'var(--success-text)', dot: 'var(--success)' },
  warning: { bg: 'var(--warning-subtle)', fg: 'var(--warning-text)', dot: 'var(--warning)' },
  danger:  { bg: 'var(--danger-subtle)', fg: 'var(--danger-text)', dot: 'var(--danger)' },
  info:    { bg: 'var(--info-subtle)', fg: 'var(--info-text)', dot: 'var(--info)' },
};

const SIZES = {
  sm: { font: 'var(--text-2xs)', pad: '2px 7px', dot: 5 },
  md: { font: 'var(--text-xs)', pad: '3px 9px', dot: 6 },
};

/**
 * Compact status / category badge. Optional leading status dot.
 */
export function Badge({ children, tone = 'neutral', size = 'md', dot = false, style, ...rest }) {
  const t = TONES[tone] || TONES.neutral;
  const sz = SIZES[size] || SIZES.md;
  return (
    <span style={{
      display: 'inline-flex', alignItems: 'center', gap: '6px',
      padding: sz.pad, borderRadius: 'var(--radius-full)',
      background: t.bg, color: t.fg,
      font: 'var(--font-sans)', fontSize: sz.font, fontWeight: 'var(--weight-semibold)',
      letterSpacing: 'var(--tracking-tight)', lineHeight: 1.4, whiteSpace: 'nowrap', ...style,
    }} {...rest}>
      {dot && <span style={{ width: sz.dot, height: sz.dot, borderRadius: '50%', background: t.dot, flex: 'none' }} />}
      {children}
    </span>
  );
}
