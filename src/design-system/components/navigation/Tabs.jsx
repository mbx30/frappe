import React from 'react';

/**
 * Tabs — controlled or uncontrolled tab strip. Two looks: `underline`
 * (page-level navigation) and `pill` (segmented, for filters/sub-views).
 */
export function Tabs({
  tabs = [], value, defaultValue, onChange, variant = 'underline', size = 'md', style, ...rest
}) {
  const isControlled = value !== undefined;
  const first = defaultValue ?? (tabs[0] && (tabs[0].value ?? tabs[0]));
  const [internal, setInternal] = React.useState(first);
  const active = isControlled ? value : internal;

  const select = (v) => {
    if (!isControlled) setInternal(v);
    onChange && onChange(v);
  };

  const fontSize = size === 'sm' ? 'var(--text-sm)' : 'var(--text-md)';
  const padY = size === 'sm' ? '6px' : '9px';

  if (variant === 'pill') {
    return (
      <div role="tablist" style={{
        display: 'inline-flex', gap: '2px', padding: '3px',
        background: 'var(--bg-subtle)', border: '1px solid var(--border-default)',
        borderRadius: 'var(--radius-md)', ...style,
      }} {...rest}>
        {tabs.map((t) => {
          const tab = typeof t === 'string' ? { value: t, label: t } : t;
          const on = tab.value === active;
          return (
            <button key={tab.value} role="tab" aria-selected={on} onClick={() => select(tab.value)}
              style={{
                display: 'inline-flex', alignItems: 'center', gap: '6px',
                padding: `${size === 'sm' ? '4px 10px' : '6px 12px'}`, border: 'none',
                borderRadius: 'var(--radius-sm)', cursor: 'pointer',
                font: 'var(--font-sans)', fontSize, fontWeight: 'var(--weight-medium)',
                background: on ? 'var(--surface-card)' : 'transparent',
                color: on ? 'var(--text-primary)' : 'var(--text-secondary)',
                boxShadow: on ? 'var(--shadow-xs)' : 'none',
                transition: 'background-color var(--duration-fast) var(--ease-standard), color var(--duration-fast) var(--ease-standard)',
              }}>
              {tab.icon}{tab.label}
              {tab.count != null && <CountPill on={on}>{tab.count}</CountPill>}
            </button>
          );
        })}
      </div>
    );
  }

  return (
    <div role="tablist" style={{
      display: 'flex', gap: '4px', borderBottom: '1px solid var(--border-default)', ...style,
    }} {...rest}>
      {tabs.map((t) => {
        const tab = typeof t === 'string' ? { value: t, label: t } : t;
        const on = tab.value === active;
        return (
          <button key={tab.value} role="tab" aria-selected={on} onClick={() => select(tab.value)}
            style={{
              display: 'inline-flex', alignItems: 'center', gap: '7px',
              padding: `${padY} 10px`, border: 'none', background: 'transparent', cursor: 'pointer',
              font: 'var(--font-sans)', fontSize, fontWeight: 'var(--weight-medium)',
              color: on ? 'var(--text-primary)' : 'var(--text-secondary)',
              borderBottom: `2px solid ${on ? 'var(--brand)' : 'transparent'}`,
              marginBottom: '-1px',
              transition: 'color var(--duration-fast) var(--ease-standard), border-color var(--duration-fast) var(--ease-standard)',
            }}>
            {tab.icon}{tab.label}
            {tab.count != null && <CountPill on={on}>{tab.count}</CountPill>}
          </button>
        );
      })}
    </div>
  );
}

function CountPill({ children, on }) {
  return (
    <span style={{
      minWidth: 18, height: 18, padding: '0 5px', borderRadius: 'var(--radius-full)',
      display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
      fontSize: 'var(--text-2xs)', fontWeight: 'var(--weight-semibold)',
      fontVariantNumeric: 'tabular-nums',
      background: on ? 'var(--brand-subtle)' : 'var(--surface-active)',
      color: on ? 'var(--brand-text)' : 'var(--text-tertiary)',
    }}>{children}</span>
  );
}
