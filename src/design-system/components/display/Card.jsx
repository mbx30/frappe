import React from 'react';

/**
 * Card — the primary surface container. Optional header (title/subtitle/
 * actions) and footer. `interactive` adds hover elevation for clickable cards.
 */
export function Card({
  title, subtitle, actions, footer, children, interactive = false,
  padding = 'md', onClick, style, bodyStyle, ...rest
}) {
  const [hover, setHover] = React.useState(false);
  const pad = { none: 0, sm: 'var(--space-8)', md: 'var(--space-12)', lg: 'var(--space-16)' }[padding] ?? 'var(--space-12)';

  return (
    <div
      onClick={onClick}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      style={{
        background: 'var(--surface-card)',
        border: '1px solid var(--border-default)',
        borderRadius: 'var(--radius-lg)',
        boxShadow: interactive && hover ? 'var(--shadow-md)' : 'var(--shadow-sm)',
        transform: interactive && hover ? 'translateY(-1px)' : 'none',
        transition: 'box-shadow var(--duration-fast) var(--ease-standard), transform var(--duration-fast) var(--ease-standard), border-color var(--duration-fast) var(--ease-standard)',
        borderColor: interactive && hover ? 'var(--border-strong)' : 'var(--border-default)',
        cursor: interactive ? 'pointer' : 'default',
        overflow: 'hidden', display: 'flex', flexDirection: 'column', ...style,
      }}
      {...rest}
    >
      {(title || actions) && (
        <div style={{
          display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: '12px',
          padding: `var(--space-12) ${typeof pad === 'string' ? pad : pad + 'px'}`,
          borderBottom: children ? '1px solid var(--border-subtle)' : 'none',
        }}>
          <div style={{ minWidth: 0 }}>
            {title && <div style={{ font: 'var(--font-title)', color: 'var(--text-primary)' }}>{title}</div>}
            {subtitle && <div style={{ font: 'var(--font-caption)', color: 'var(--text-tertiary)', marginTop: '2px' }}>{subtitle}</div>}
          </div>
          {actions && <div style={{ flex: 'none', display: 'flex', gap: '6px' }}>{actions}</div>}
        </div>
      )}
      {children != null && <div style={{ padding: pad, flex: 1, ...bodyStyle }}>{children}</div>}
      {footer && (
        <div style={{
          padding: `var(--space-10) ${typeof pad === 'string' ? pad : pad + 'px'}`,
          borderTop: '1px solid var(--border-subtle)', background: 'var(--bg-subtle)',
        }}>{footer}</div>
      )}
    </div>
  );
}
