/* App shell — sidebar, topbar, theme toggle */
(function () {
  const { Avatar, Badge, IconButton } = window.FrappeDesignSystem_75694f;
  const Tooltip = window.FrappeDesignSystem_75694f.Tooltip || (({ children }) => children);
  const { Ic } = window.FK;

  const NAV_GROUPS = [
    {
      label: 'Shop',
      items: [
        { key: 'dashboard', label: 'Dashboard', icon: 'LayoutDashboard' },
        { key: 'orders', label: 'Orders', icon: 'ClipboardList', count: 7 },
        { key: 'production', label: 'Production', icon: 'Factory', count: 4 },
        { key: 'estimates', label: 'Estimates', icon: 'FileText' },
        { key: 'invoices', label: 'Invoices', icon: 'Receipt', count: 2 },
      ],
    },
    {
      label: 'Manage',
      items: [
        { key: 'clients', label: 'Clients', icon: 'Users' },
        { key: 'inventory', label: 'Inventory', icon: 'Boxes' },
        { key: 'workbooks', label: 'Workbooks', icon: 'BookOpen' },
      ],
    },
    {
      label: 'Finance & Tools',
      items: [
        { key: 'pos', label: 'Point of Sale', icon: 'CreditCard' },
        { key: 'qb', label: 'QuickBooks', icon: 'Zap' },
        { key: 'pdf', label: 'PDF Tools', icon: 'FileSearch' },
      ],
    },
  ];

  function NavItem({ item, active, onClick }) {
    const [hover, setHover] = React.useState(false);
    return (
      <button onClick={onClick} onMouseEnter={() => setHover(true)} onMouseLeave={() => setHover(false)}
        style={{ display: 'flex', alignItems: 'center', gap: '10px', width: '100%', padding: '7px 10px', border: 'none', cursor: 'pointer', textAlign: 'left',
          borderRadius: 'var(--radius-md)',
          fontFamily: 'var(--font-sans)', fontWeight: 500, fontSize: 'var(--text-md)', lineHeight: 1.5,
          backgroundColor: active ? 'var(--brand-subtle)' : hover ? 'var(--surface-hover)' : 'transparent',
          color: active ? 'var(--brand-text)' : 'var(--text-secondary)',
          transition: 'background-color var(--duration-fast), color var(--duration-fast)' }}>
        <span style={{ display: 'inline-flex', color: active ? 'var(--brand)' : 'var(--text-tertiary)', flexShrink: 0 }}><Ic n={item.icon} size={16} /></span>
        <span style={{ flex: 1 }}>{item.label}</span>
        {item.count != null && <Badge tone={active ? 'brand' : 'neutral'} size="sm">{item.count}</Badge>}
      </button>
    );
  }

  function Sidebar({ view, setView }) {
    return (
      <aside style={{ width: '200px', flexShrink: 0, background: 'var(--surface-card)', borderRight: '1px solid var(--border-default)', display: 'flex', flexDirection: 'column', height: '100%' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '9px', padding: '13px 14px', height: '48px', boxSizing: 'border-box', borderBottom: '1px solid var(--border-subtle)' }}>
          <img src={window.__resources && window.__resources.frappeLogoSvg ? window.__resources.frappeLogoSvg : "../../assets/frappe-logo.svg"} width="22" height="21" alt="Frappe" style={{ display: 'block' }} />
          <span style={{ font: 'var(--font-title)', fontWeight: 700, color: 'var(--text-primary)', letterSpacing: 'var(--tracking-tight)' }}>Frappe</span>
          <span style={{ marginLeft: 'auto', font: 'var(--font-caption)', color: 'var(--text-tertiary)' }}>⌘K</span>
        </div>

        <div style={{ padding: '8px', display: 'flex', flexDirection: 'column', gap: '0', flex: 1, overflowY: 'auto' }}>
          {NAV_GROUPS.map((group, gi) => (
            <div key={group.label} style={{ marginBottom: gi < NAV_GROUPS.length - 1 ? '4px' : 0 }}>
              <div style={{ font: 'var(--font-caption)', fontWeight: 600, textTransform: 'uppercase', letterSpacing: 'var(--tracking-caps)', color: 'var(--text-tertiary)', padding: '10px 10px 4px' }}>{group.label}</div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '1px' }}>
                {group.items.map((item) => (
                  <NavItem key={item.key} item={item} active={view === item.key} onClick={() => setView(item.key)} />
                ))}
              </div>
            </div>
          ))}
        </div>

        <div style={{ padding: '8px', borderTop: '1px solid var(--border-subtle)' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '9px', padding: '6px 8px', borderRadius: 'var(--radius-md)', cursor: 'pointer' }}
            onMouseEnter={(e) => e.currentTarget.style.background = 'var(--surface-hover)'}
            onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}>
            <Avatar name="Max Bowen" size="sm" />
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ font: 'var(--font-label)', color: 'var(--text-primary)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>Max Bowen</div>
              <div style={{ font: 'var(--font-caption)', color: 'var(--text-tertiary)' }}>Bowen Print Co.</div>
            </div>
            <span style={{ color: 'var(--text-tertiary)', flexShrink: 0 }}><Ic n="ChevronsUpDown" size={14} /></span>
          </div>
        </div>
      </aside>
    );
  }

  function Topbar({ theme, toggleTheme }) {
    return (
      <header style={{ height: '48px', flexShrink: 0, borderBottom: '1px solid var(--border-default)', background: 'var(--surface-card)', display: 'flex', alignItems: 'center', gap: '12px', padding: '0 16px' }}>
        <div style={{ position: 'relative', width: '280px', maxWidth: '40vw' }}>
          <span style={{ position: 'absolute', left: '9px', top: '50%', transform: 'translateY(-50%)', color: 'var(--text-tertiary)', pointerEvents: 'none', display: 'flex' }}><Ic n="Search" size={14} /></span>
          <input placeholder="Search orders, clients, invoices…"
            style={{ width: '100%', height: '32px', boxSizing: 'border-box', padding: '0 10px 0 30px', borderRadius: 'var(--radius-md)', border: '1px solid var(--border-default)', background: 'var(--surface-inset)', color: 'var(--text-primary)', font: 'var(--font-body)', outline: 'none' }} />
        </div>
        <div style={{ flex: 1 }} />
        <Tooltip label="Sync with QuickBooks"><IconButton icon={<Ic n="RefreshCw" size={15} />} label="Sync" variant="ghost" /></Tooltip>
        <Tooltip label="Notifications"><IconButton icon={<Ic n="Bell" size={15} />} label="Notifications" variant="ghost" /></Tooltip>
        <Tooltip label={theme === 'dark' ? 'Light mode' : 'Dark mode'}>
          <IconButton icon={<Ic n={theme === 'dark' ? 'Sun' : 'Moon'} size={15} />} label="Toggle theme" variant="ghost" onClick={toggleTheme} />
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
          <main style={{ flex: 1, overflowY: 'auto', padding: '22px 26px' }}>
            <div style={{ maxWidth: '1200px', margin: '0 auto' }}>{children}</div>
          </main>
        </div>
      </div>
    );
  }

  window.AppShell = AppShell;
})();
