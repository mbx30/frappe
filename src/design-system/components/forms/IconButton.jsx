import React from 'react';

const SIZES = {
  sm: { box: 'var(--control-sm)', radius: 'var(--radius-sm)' },
  md: { box: 'var(--control-md)', radius: 'var(--radius-md)' },
  lg: { box: 'var(--control-lg)', radius: 'var(--radius-md)' },
};

const VARIANTS = {
  secondary: { background: 'var(--surface-card)', color: 'var(--text-secondary)', border: '1px solid var(--border-default)', '--hover-bg': 'var(--surface-hover)' },
  ghost: { background: 'transparent', color: 'var(--text-secondary)', border: '1px solid transparent', '--hover-bg': 'var(--surface-hover)' },
  subtle: { background: 'var(--brand-subtle)', color: 'var(--brand-text)', border: '1px solid transparent', '--hover-bg': 'var(--brand-subtle-hover)' },
  primary: { background: 'var(--brand)', color: 'var(--text-on-brand)', border: '1px solid transparent', '--hover-bg': 'var(--brand-hover)' },
};

/**
 * Square icon-only button. Pairs a single glyph with an accessible label.
 */
export function IconButton({
  icon, label, variant = 'ghost', size = 'md', disabled = false, onClick, style, ...rest
}) {
  const sz = SIZES[size] || SIZES.md;
  const vr = VARIANTS[variant] || VARIANTS.ghost;
  const [hover, setHover] = React.useState(false);
  const [active, setActive] = React.useState(false);

  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={disabled ? undefined : onClick}
      disabled={disabled}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => { setHover(false); setActive(false); }}
      onMouseDown={() => setActive(true)}
      onMouseUp={() => setActive(false)}
      style={{
        display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
        width: sz.box, height: sz.box, padding: 0,
        borderRadius: sz.radius, border: vr.border,
        background: hover && !disabled ? vr['--hover-bg'] : vr.background,
        color: vr.color, cursor: disabled ? 'not-allowed' : 'pointer',
        opacity: disabled ? 0.5 : 1,
        transition: 'background-color var(--duration-fast) var(--ease-standard), transform var(--duration-instant) var(--ease-standard)',
        transform: active && !disabled ? 'scale(0.94)' : 'scale(1)',
        ...style,
      }}
      {...rest}
    >
      {icon}
    </button>
  );
}
