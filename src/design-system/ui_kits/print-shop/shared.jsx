/* Shared atoms for the print-shop kit. Exposes window.FK = { Ic, ... } */
(function () {
  const { useState } = React;

  // Lucide icon → React, themed via currentColor
  function Ic({ n, size = 16, style }) {
    const node = window.lucide && window.lucide[n];
    if (!node) return null;
    const svg = window.lucide.createElement(node);
    svg.setAttribute('width', size);
    svg.setAttribute('height', size);
    return <span style={{ display: 'inline-flex', ...style }} dangerouslySetInnerHTML={{ __html: svg.outerHTML }} />;
  }

  // Tiny uppercase eyebrow label
  function Eyebrow({ children, style }) {
    return <div style={{ font: 'var(--font-caption)', fontWeight: 600, textTransform: 'uppercase', letterSpacing: 'var(--tracking-caps)', color: 'var(--text-tertiary)', ...style }}>{children}</div>;
  }

  // KPI stat card for the dashboard
  function Kpi({ label, value, delta, deltaTone = 'success', icon }) {
    return (
      <div style={{ flex: 1, minWidth: 0, background: 'var(--surface-card)', border: '1px solid var(--border-default)', borderRadius: 'var(--radius-lg)', boxShadow: 'var(--shadow-sm)', padding: '16px 18px' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '10px' }}>
          <Eyebrow>{label}</Eyebrow>
          <span style={{ color: 'var(--text-tertiary)' }}><Ic n={icon} size={16} /></span>
        </div>
        <div style={{ font: 'var(--font-h1)', fontFamily: 'var(--font-mono)', letterSpacing: '-0.02em', color: 'var(--text-primary)' }} className="tabular">{value}</div>
        {delta && (
          <div style={{ marginTop: '6px', font: 'var(--font-caption)', fontWeight: 500, color: deltaTone === 'success' ? 'var(--success-text)' : deltaTone === 'danger' ? 'var(--danger-text)' : 'var(--text-secondary)' }}>{delta}</div>
        )}
      </div>
    );
  }

  // Page title row
  function PageHeader({ title, subtitle, actions }) {
    return (
      <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: '16px', marginBottom: '20px' }}>
        <div>
          <h1 style={{ margin: 0, font: 'var(--font-h2)', color: 'var(--text-primary)', letterSpacing: 'var(--tracking-tight)' }}>{title}</h1>
          {subtitle && <div style={{ marginTop: '4px', font: 'var(--font-body)', color: 'var(--text-secondary)' }}>{subtitle}</div>}
        </div>
        {actions && <div style={{ display: 'flex', gap: '8px', flexShrink: 0 }}>{actions}</div>}
      </div>
    );
  }

  window.FK = { Ic, Eyebrow, Kpi, PageHeader };
})();
