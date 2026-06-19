import React from 'react';

/**
 * Checkbox with label. Supports indeterminate via the `indeterminate` prop.
 */
export function Checkbox({
  checked, defaultChecked, indeterminate = false, onChange, label, hint,
  disabled = false, id, ...rest
}) {
  const boxId = id || React.useId();
  const isControlled = checked !== undefined;
  const [internal, setInternal] = React.useState(!!defaultChecked);
  const on = isControlled ? checked : internal;

  const toggle = (e) => {
    if (disabled) return;
    if (!isControlled) setInternal(e.target.checked);
    onChange && onChange(e);
  };

  return (
    <label htmlFor={boxId} style={{
      display: 'inline-flex', alignItems: hint ? 'flex-start' : 'center', gap: '9px',
      cursor: disabled ? 'not-allowed' : 'pointer', opacity: disabled ? 0.55 : 1,
    }}>
      <span style={{ position: 'relative', display: 'inline-flex', flex: 'none', marginTop: hint ? '1px' : 0 }}>
        <input
          type="checkbox" id={boxId} checked={isControlled ? checked : undefined}
          defaultChecked={isControlled ? undefined : defaultChecked}
          onChange={toggle} disabled={disabled}
          style={{ position: 'absolute', opacity: 0, width: '18px', height: '18px', margin: 0, cursor: 'inherit' }}
          {...rest}
        />
        <span style={{
          width: '18px', height: '18px', borderRadius: 'var(--radius-xs)',
          border: `1.5px solid ${on || indeterminate ? 'var(--brand)' : 'var(--border-strong)'}`,
          background: on || indeterminate ? 'var(--brand)' : 'var(--surface-inset)',
          display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
          transition: 'background-color var(--duration-fast) var(--ease-standard), border-color var(--duration-fast) var(--ease-standard)',
          color: '#fff',
        }}>
          {indeterminate ? (
            <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3.5" strokeLinecap="round"><path d="M5 12h14" /></svg>
          ) : on ? (
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3.2" strokeLinecap="round" strokeLinejoin="round"><path d="M20 6 9 17l-5-5" /></svg>
          ) : null}
        </span>
      </span>
      {label && (
        <span style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
          <span style={{ font: 'var(--font-body)', color: 'var(--text-primary)' }}>{label}</span>
          {hint && <span style={{ font: 'var(--font-caption)', color: 'var(--text-tertiary)' }}>{hint}</span>}
        </span>
      )}
    </label>
  );
}
