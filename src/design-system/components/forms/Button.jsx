import React from 'react';

const SIZES = {
  sm: { height: 'var(--control-sm)', padding: '0 10px', font: 'var(--text-sm)', gap: '6px', radius: 'var(--radius-sm)' },
  md: { height: 'var(--control-md)', padding: '0 14px', font: 'var(--text-md)', gap: '7px', radius: 'var(--radius-md)' },
  lg: { height: 'var(--control-lg)', padding: '0 18px', font: 'var(--text-base)', gap: '8px', radius: 'var(--radius-md)' },
};

const VARIANTS = {
  primary: {
    background: 'var(--brand)', color: 'var(--text-on-brand)',
    border: '1px solid transparent', boxShadow: 'var(--shadow-xs)',
    '--hover-bg': 'var(--brand-hover)', '--active-bg': 'var(--brand-active)',
  },
  secondary: {
    background: 'var(--surface-card)', color: 'var(--text-primary)',
    border: '1px solid var(--border-default)', boxShadow: 'var(--shadow-xs)',
    '--hover-bg': 'var(--surface-hover)', '--active-bg': 'var(--surface-active)',
  },
  subtle: {
    background: 'var(--brand-subtle)', color: 'var(--brand-text)',
    border: '1px solid transparent',
    '--hover-bg': 'var(--brand-subtle-hover)', '--active-bg': 'var(--brand-subtle-hover)',
  },
  ghost: {
    background: 'transparent', color: 'var(--text-secondary)',
    border: '1px solid transparent',
    '--hover-bg': 'var(--surface-hover)', '--active-bg': 'var(--surface-active)',
  },
  danger: {
    background: 'var(--danger)', color: '#fff',
    border: '1px solid transparent', boxShadow: 'var(--shadow-xs)',
    '--hover-bg': 'var(--danger)', '--active-bg': 'var(--danger)',
  },
};

const Spinner = () => (
  <span style={{
    width: '1em', height: '1em', borderRadius: '50%',
    border: '2px solid currentColor', borderTopColor: 'transparent',
    display: 'inline-block', animation: 'frappe-spin 0.6s linear infinite', opacity: 0.9,
  }} />
);

/**
 * Frappe primary action button. Five variants, three sizes.
 */
export function Button({
  children, variant = 'secondary', size = 'md', type = 'button',
  iconLeft, iconRight, loading = false, disabled = false, fullWidth = false,
  onClick, style, ...rest
}) {
  const sz = SIZES[size] || SIZES.md;
  const vr = VARIANTS[variant] || VARIANTS.secondary;
  const isDisabled = disabled || loading;
  const [hover, setHover] = React.useState(false);
  const [active, setActive] = React.useState(false);

  const bg = active ? vr['--active-bg'] : hover ? vr['--hover-bg'] : vr.background;

  return (
    <button
      type={type}
      onClick={isDisabled ? undefined : onClick}
      disabled={isDisabled}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => { setHover(false); setActive(false); }}
      onMouseDown={() => setActive(true)}
      onMouseUp={() => setActive(false)}
      style={{
        display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
        gap: sz.gap, height: sz.height, padding: sz.padding,
        fontFamily: 'var(--font-sans)', fontSize: sz.font, fontWeight: 'var(--weight-medium)',
        lineHeight: 1, letterSpacing: 'var(--tracking-tight)',
        borderRadius: sz.radius, cursor: isDisabled ? 'not-allowed' : 'pointer',
        width: fullWidth ? '100%' : 'auto',
        transition: 'background-color var(--duration-fast) var(--ease-standard), transform var(--duration-instant) var(--ease-standard), box-shadow var(--duration-fast) var(--ease-standard)',
        transform: active && !isDisabled ? 'scale(0.97)' : 'scale(1)',
        opacity: isDisabled ? 0.5 : 1,
        boxShadow: vr.boxShadow, color: vr.color, border: vr.border,
        background: bg, whiteSpace: 'nowrap', userSelect: 'none', ...style,
      }}
      {...rest}
    >
      <style>{'@keyframes frappe-spin{to{transform:rotate(360deg)}}'}</style>
      {loading ? <Spinner /> : iconLeft}
      {children && <span>{children}</span>}
      {!loading && iconRight}
    </button>
  );
}
