import React from 'react';

const SIZES = {
  sm: { height: 'var(--control-sm)', font: 'var(--text-sm)', pad: '0 30px 0 10px' },
  md: { height: 'var(--control-md)', font: 'var(--text-md)', pad: '0 32px 0 12px' },
  lg: { height: 'var(--control-lg)', font: 'var(--text-base)', pad: '0 34px 0 14px' },
};

/**
 * Styled native select with label, hint/error and a chevron affordance.
 */
export function Select({
  label, hint, error, size = 'md', options = [], placeholder,
  disabled = false, id, value, onChange, containerStyle, style, children, ...rest
}) {
  const sz = SIZES[size] || SIZES.md;
  const [focus, setFocus] = React.useState(false);
  const selectId = id || React.useId();
  const borderColor = error ? 'var(--danger)' : focus ? 'var(--brand)' : 'var(--border-default)';

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', ...containerStyle }}>
      {label && (
        <label htmlFor={selectId} style={{ font: 'var(--font-label)', color: 'var(--text-secondary)' }}>{label}</label>
      )}
      <div style={{ position: 'relative', display: 'flex', alignItems: 'center' }}>
        <select
          id={selectId}
          value={value}
          onChange={onChange}
          disabled={disabled}
          onFocus={() => setFocus(true)}
          onBlur={() => setFocus(false)}
          style={{
            appearance: 'none', WebkitAppearance: 'none',
            width: '100%', height: sz.height, padding: sz.pad,
            fontFamily: 'var(--font-sans)', fontSize: sz.font, color: 'var(--text-primary)',
            background: 'var(--surface-inset)', border: `1px solid ${borderColor}`,
            borderRadius: 'var(--radius-md)', cursor: disabled ? 'not-allowed' : 'pointer',
            boxShadow: focus ? '0 0 0 3px var(--focus-ring)' : 'none',
            transition: 'border-color var(--duration-fast) var(--ease-standard), box-shadow var(--duration-fast) var(--ease-standard)',
            opacity: disabled ? 0.55 : 1, ...style,
          }}
          {...rest}
        >
          {placeholder && <option value="" disabled>{placeholder}</option>}
          {options.map((o) => {
            const opt = typeof o === 'string' ? { value: o, label: o } : o;
            return <option key={opt.value} value={opt.value}>{opt.label}</option>;
          })}
          {children}
        </select>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2"
          strokeLinecap="round" strokeLinejoin="round"
          style={{ position: 'absolute', right: '11px', color: 'var(--text-tertiary)', pointerEvents: 'none' }}>
          <path d="m6 9 6 6 6-6" />
        </svg>
      </div>
      {(hint || error) && (
        <span style={{ font: 'var(--font-caption)', color: error ? 'var(--danger-text)' : 'var(--text-tertiary)' }}>
          {error || hint}
        </span>
      )}
    </div>
  );
}
