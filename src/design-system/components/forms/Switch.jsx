import React from 'react';

const SIZES = {
  sm: { w: 32, h: 18, knob: 12 },
  md: { w: 40, h: 22, knob: 16 },
};

/**
 * On/off switch. Use for instant-apply settings (not form submission).
 */
export function Switch({
  checked, defaultChecked, onChange, label, size = 'md', disabled = false, id, ...rest
}) {
  const sz = SIZES[size] || SIZES.md;
  const switchId = id || React.useId();
  const isControlled = checked !== undefined;
  const [internal, setInternal] = React.useState(!!defaultChecked);
  const on = isControlled ? checked : internal;

  const toggle = (e) => {
    if (disabled) return;
    if (!isControlled) setInternal(e.target.checked);
    onChange && onChange(e);
  };

  const pad = (sz.h - sz.knob) / 2;

  return (
    <label htmlFor={switchId} style={{
      display: 'inline-flex', alignItems: 'center', gap: '10px',
      cursor: disabled ? 'not-allowed' : 'pointer', opacity: disabled ? 0.55 : 1,
    }}>
      <span style={{ position: 'relative', display: 'inline-flex', flex: 'none' }}>
        <input type="checkbox" id={switchId} checked={isControlled ? checked : undefined}
          defaultChecked={isControlled ? undefined : defaultChecked}
          onChange={toggle} disabled={disabled}
          style={{ position: 'absolute', opacity: 0, width: sz.w, height: sz.h, margin: 0, cursor: 'inherit' }}
          {...rest}
        />
        <span style={{
          width: sz.w, height: sz.h, borderRadius: 'var(--radius-full)',
          background: on ? 'var(--brand)' : 'var(--border-strong)',
          transition: 'background-color var(--duration-base) var(--ease-standard)',
          display: 'inline-block', position: 'relative',
        }}>
          <span style={{
            position: 'absolute', top: pad, left: pad, width: sz.knob, height: sz.knob,
            borderRadius: '50%', background: '#fff', boxShadow: 'var(--shadow-sm)',
            transform: on ? `translateX(${sz.w - sz.knob - pad * 2}px)` : 'translateX(0)',
            transition: 'transform var(--duration-base) var(--ease-out)',
          }} />
        </span>
      </span>
      {label && <span style={{ font: 'var(--font-body)', color: 'var(--text-primary)' }}>{label}</span>}
    </label>
  );
}
