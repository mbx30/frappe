/* App shell — sidebar, topbar, theme toggle */
(function () {
  const { Avatar, Badge, IconButton } = window.FrappeDesignSystem_75694f;
  const Tooltip = window.FrappeDesignSystem_75694f.Tooltip || (({ children }) => children);
  const { Ic } = window.FK;

  const NAV = [
    { key: 'dashboard', label: 'Dashboard', icon: 'LayoutDashboard' },
    { key: 'orders', label: 'Orders', icon: 'ClipboardList', count: 18 },
    { key: 'production', label: 'Production', icon: 'Factory', count: 7 },
    { key: 'invoicing', label: 'Invoicing', icon: 'Receipt', count: 3 },
    { key: 'inventory', label: 'Inventory', icon: 'Boxes' },
    { key: 'customers', label: 'Customers', icon: 'Users' },
  ];

  function NavItem({ item, active, onClick }) {
    const [hover, setHover] = React.useState(false);
    return (
      <button key={active ? 'on' : 'off'} onClick={onClick} onMouseEnter={() => setHover(true)} onMouseLeave={() => setHover(false)}
        style={{ display: 'flex', alignItems: 'center', gap: '10px', width: '100%', padding: '8px 10px', border: 'none', cursor: 'pointer', textAlign: 'left',
          borderRadius: 'var(--radius-md)',
          fontFamily: 'var(--font-sans)', fontWeight: 500, fontSize: 'var(--text-md)', lineHeight: 1.5,
          backgroundColor: active ? 'var(--brand-subtle)' : hover ? 'var(--surface-hover)' : 'transparent',
          color: active ? 'var(--brand-text)' : 'var(--text-secondary)',
          transition: 'background-color var(--duration-fast), color var(--duration-fast)' }}>
        <span style={{ display: 'inline-flex', color: active ? 'var(--brand)' : 'var(--text-tertiary)' }}><Ic n={item.icon} size={17} /></span>
        <span style={{ flex: 1 }}>{item.label}</span>
        {item.count != null && <Badge tone={active ? 'brand' : 'neutral'} size="sm">{item.count}</Badge>}
      </button>
    );
  }

  function Sidebar({ view, setView }) {
    return (
      <aside style={{ width: 'var(--sidebar-width)', flexShrink: 0, background: 'var(--surface-card)', borderRight: '1px solid var(--border-default)', display: 'flex', flexDirection: 'column', height: '100%' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '9px', padding: '14px 16px', height: 'var(--topbar-height)', boxSizing: 'border-box', borderBottom: '1px solid var(--border-subtle)' }}>
          <img src="../../assets/frappe-logo.svg" width="24" height="23" alt="Frappe" style={{ display: 'block' }} />
          <span style={{ font: 'var(--font-title)', fontWeight: 700, color: 'var(--text-primary)', letterSpacing: 'var(--tracking-tight)' }}>Frappe</span>
          <span style={{ marginLeft: 'auto', font: 'var(--font-caption)', color: 'var(--text-tertiary)' }}>⌘K</span>
        </div>

        <div style={{ padding: '10px', display: 'flex', flexDirection: 'column', gap: '2px', flex: 1, overflowY: 'auto' }}>
          <div style={{ font: 'var(--font-caption)', fontWeight: 600, textTransform: 'uppercase', letterSpacing: 'var(--tracking-caps)', color: 'var(--text-tertiary)', padding: '8px 10px 4px' }}>Shop</div>
          {NAV.map((item) => <NavItem key={item.key} item={item} active={view === item.key} onClick={() => setView(item.key)} />)}
        </div>

        <div style={{ padding: '10px', borderTop: '1px solid var(--border-subtle)' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '9px', padding: '6px 8px' }}>
            <Avatar name="Max Bowen" size="sm" />
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ font: 'var(--font-label)', color: 'var(--text-primary)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>Max Bowen</div>
              <div style={{ font: 'var(--font-caption)', color: 'var(--text-tertiary)' }}>Bowen Print Co.</div>
            </div>
            <span style={{ color: 'var(--text-tertiary)' }}><Ic n="ChevronsUpDown" size={15} /></span>
          </div>
        </div>
      </aside>
    );
  }

  function Topbar({ theme, toggleTheme }) {
    return (
      <header style={{ height: 'var(--topbar-height)', flexShrink: 0, borderBottom: '1px solid var(--border-default)', background: 'var(--surface-card)', display: 'flex', alignItems: 'center', gap: '12px', padding: '0 18px' }}>
        <div style={{ position: 'relative', width: '320px', maxWidth: '40vw' }}>
          <span style={{ position: 'absolute', left: '10px', top: '50%', transform: 'translateY(-50%)', color: 'var(--text-tertiary)', pointerEvents: 'none', display: 'flex' }}><Ic n="Search" size={15} /></span>
          <input placeholder="Search orders, customers, invoices…"
            style={{ width: '100%', height: 'var(--control-md)', boxSizing: 'border-box', padding: '0 10px 0 32px', borderRadius: 'var(--radius-md)', border: '1px solid var(--border-default)', background: 'var(--surface-inset)', color: 'var(--text-primary)', font: 'var(--font-body)', outline: 'none' }} />
        </div>
        <div style={{ flex: 1 }} />
        <Tooltip label="Sync with QuickBooks"><IconButton icon={<Ic n="RefreshCw" size={16} />} label="Sync" variant="ghost" /></Tooltip>
        <Tooltip label="Notifications"><IconButton icon={<Ic n="Bell" size={16} />} label="Notifications" variant="ghost" /></Tooltip>
        <Tooltip label={theme === 'dark' ? 'Light mode' : 'Dark mode'}>
          <IconButton icon={<Ic n={theme === 'dark' ? 'Sun' : 'Moon'} size={16} />} label="Toggle theme" variant="ghost" onClick={toggleTheme} />
        </Tooltip>
      </header>
    );
  }

  function AppShell({ view, setView, theme, toggleTheme, children }) {
    return (
      <div style={{ display: 'flex', height: '100vh', width: '100vw', overflow: 'hidden', background: 'var(--bg-base)' }}>
        <Sidebar view={view} setView={setView} />
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
          <Topbar theme={theme} toggleTheme={toggleTheme} />
          <main style={{ flex: 1, overflowY: 'auto', padding: '24px 28px' }}>
            <div style={{ maxWidth: '1200px', margin: '0 auto' }}>{children}</div>
          </main>
        </div>
      </div>
    );
  }

  window.AppShell = AppShell;
})();
