import React from 'react';

const SIZES = {
  sm: { height: 'var(--control-sm)', font: 'var(--text-sm)', pad: '0 10px' },
  md: { height: 'var(--control-md)', font: 'var(--text-md)', pad: '0 12px' },
  lg: { height: 'var(--control-lg)', font: 'var(--text-base)', pad: '0 14px' },
};

/**
 * Text input with optional label, leading icon, suffix, and error state.
 */
export function Input({
  label, hint, error, size = 'md', iconLeft, suffix, mono = false,
  disabled = false, id, style, containerStyle, ...rest
}) {
  const sz = SIZES[size] || SIZES.md;
  const [focus, setFocus] = React.useState(false);
  const inputId = id || React.useId();
  const borderColor = error ? 'var(--danger)' : focus ? 'var(--brand)' : 'var(--border-default)';

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', ...containerStyle }}>
      {label && (
        <label htmlFor={inputId} style={{
          font: 'var(--font-label)', color: 'var(--text-secondary)',
        }}>{label}</label>
      )}
      <div style={{
        display: 'flex', alignItems: 'center', gap: '8px',
        height: sz.height, padding: sz.pad,
        background: 'var(--surface-inset)',
        border: `1px solid ${borderColor}`,
        borderRadius: 'var(--radius-md)',
        boxShadow: focus ? `0 0 0 3px var(--focus-ring)` : 'none',
        transition: 'border-color var(--duration-fast) var(--ease-standard), box-shadow var(--duration-fast) var(--ease-standard)',
        opacity: disabled ? 0.55 : 1,
        cursor: disabled ? 'not-allowed' : 'text',
      }}>
        {iconLeft && <span style={{ display: 'flex', color: 'var(--text-tertiary)', flex: 'none' }}>{iconLeft}</span>}
        <input
          id={inputId}
          disabled={disabled}
          onFocus={() => setFocus(true)}
          onBlur={() => setFocus(false)}
          style={{
            flex: 1, minWidth: 0, border: 'none', outline: 'none', background: 'transparent',
            fontFamily: mono ? 'var(--font-mono)' : 'var(--font-sans)',
            fontSize: sz.font, color: 'var(--text-primary)',
            fontVariantNumeric: mono ? 'tabular-nums' : 'normal',
            ...style,
          }}
          {...rest}
        />
        {suffix && <span style={{ font: 'var(--font-caption)', color: 'var(--text-tertiary)', flex: 'none' }}>{suffix}</span>}
      </div>
      {(hint || error) && (
        <span style={{ font: 'var(--font-caption)', color: error ? 'var(--danger-text)' : 'var(--text-tertiary)' }}>
          {error || hint}
        </span>
      )}
    </div>
  );
}
