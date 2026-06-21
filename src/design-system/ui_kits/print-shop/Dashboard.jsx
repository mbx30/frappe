/* Dashboard screen */
(function () {
  const { useState } = React;
  const { Card, Badge, Avatar, AvatarGroup, Button, Input, Select } = window.FrappeDesignSystem_75694f;
  const { Ic, Eyebrow, Kpi, PageHeader } = window.FK;
  const D = window.FrappeData;

  const STATUS_OPTS = [
    { value: '', label: 'All status' },
    { value: 'press',   label: 'On press' },
    { value: 'art',     label: 'Awaiting art' },
    { value: 'queued',  label: 'Queued' },
    { value: 'shipped', label: 'Shipped' },
    { value: 'overdue', label: 'Overdue' },
  ];
  const PRIORITY_OPTS = [
    { value: '', label: 'All priority' },
    { value: 'rush',   label: 'Rush' },
    { value: 'normal', label: 'Normal' },
  ];

  function Dashboard({ onOpenOrder }) {
    const [search,    setSearch]    = useState('');
    const [statusF,   setStatusF]   = useState('');
    const [priorityF, setPriorityF] = useState('');

    const q = search.toLowerCase();
    const filtered = D.orders.filter(o => {
      if (q && !String(o.id).includes(q) && !o.job.toLowerCase().includes(q) && !o.customer.toLowerCase().includes(q)) return false;
      if (statusF   && o.status !== statusF)               return false;
      if (priorityF && (priorityF === 'rush') !== !!o.rush) return false;
      return true;
    }).slice(0, 6);

    const hasFilters = search || statusF || priorityF;

    return (
      <div>
        <PageHeader
          title="Good morning, Max"
          subtitle="Wednesday, June 19 · 7 jobs on the floor, 2 due today"
          actions={<Button variant="primary" iconLeft={<Ic n="Plus" size={15} />}>New order</Button>}
        />

        {/* Filter bar */}
        <div style={{ display: 'flex', gap: '8px', alignItems: 'flex-end', marginBottom: '18px', flexWrap: 'wrap' }}>
          <div style={{ flex: '1 1 180px', minWidth: '140px' }}>
            <Input placeholder="Search orders…" value={search} onChange={e => setSearch(e.target.value)} iconLeft={<Ic n="Search" size={13} />} />
          </div>
          <div style={{ width: '148px' }}>
            <Select value={statusF} onChange={e => setStatusF(e.target.value)} options={STATUS_OPTS} />
          </div>
          <div style={{ width: '136px' }}>
            <Select value={priorityF} onChange={e => setPriorityF(e.target.value)} options={PRIORITY_OPTS} />
          </div>
          {hasFilters && <Button variant="secondary" size="sm" onClick={() => { setSearch(''); setStatusF(''); setPriorityF(''); }}>Clear</Button>}
        </div>

        <div style={{ display: 'flex', gap: '14px', marginBottom: '22px' }}>
          <Kpi label="Open orders" value="18" delta="+3 this week" icon="ClipboardList" />
          <Kpi label="Due today" value="2" delta="1 rush" deltaTone="danger" icon="CalendarClock" />
          <Kpi label="Awaiting art" value="4" delta="Oldest 2d" deltaTone="neutral" icon="Image" />
          <Kpi label="Revenue (MTD)" value="$24.6k" delta="+12% vs May" icon="TrendingUp" />
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: '1.7fr 1fr', gap: '16px', alignItems: 'start' }}>
          <Card padding="none">
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '14px 16px', borderBottom: '1px solid var(--border-subtle)' }}>
              <div style={{ font: 'var(--font-title)', color: 'var(--text-primary)' }}>Recent orders</div>
              <Button variant="ghost" size="sm" iconRight={<Ic n="ArrowRight" size={14} />}>View all</Button>
            </div>
            <table style={{ width: '100%', borderCollapse: 'collapse' }}>
              <thead>
                <tr>
                  {['Order', 'Customer', 'Job', 'Status', 'Total'].map((h, i) => (
                    <th key={h} style={{ textAlign: i === 4 ? 'right' : 'left', font: 'var(--font-caption)', fontWeight: 600, textTransform: 'uppercase', letterSpacing: 'var(--tracking-caps)', color: 'var(--text-tertiary)', padding: '8px 16px', borderBottom: '1px solid var(--border-subtle)' }}>{h}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {filtered.length === 0 && (
                  <tr><td colSpan={5} style={{ padding: '24px 16px', textAlign: 'center', font: 'var(--font-body)', color: 'var(--text-tertiary)' }}>No orders match the current filters.</td></tr>
                )}
                {filtered.map((o) => {
                  const b = D.statusBadge[o.status];
                  return (
                    <tr key={o.id} onClick={() => onOpenOrder(o)} style={{ cursor: 'pointer', transition: 'background var(--duration-fast)' }}
                      onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--surface-hover)')}
                      onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}>
                      <td style={{ padding: '11px 16px', borderBottom: '1px solid var(--border-subtle)', fontFamily: 'var(--font-mono)', fontWeight: 500, color: 'var(--text-primary)' }} className="tabular">#{o.id}</td>
                      <td style={{ padding: '11px 16px', borderBottom: '1px solid var(--border-subtle)', font: 'var(--font-body-strong)', color: 'var(--text-primary)' }}>{o.customer}</td>
                      <td style={{ padding: '11px 16px', borderBottom: '1px solid var(--border-subtle)', font: 'var(--font-body)', color: 'var(--text-secondary)', maxWidth: '180px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{o.job}</td>
                      <td style={{ padding: '11px 16px', borderBottom: '1px solid var(--border-subtle)' }}><Badge tone={b.tone} dot size="sm">{b.label}</Badge></td>
                      <td style={{ padding: '11px 16px', borderBottom: '1px solid var(--border-subtle)', textAlign: 'right', fontFamily: 'var(--font-mono)', fontWeight: 500, color: 'var(--text-primary)' }} className="tabular">{D.money(o.total)}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </Card>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
            <Card title="On the floor">
              <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                {D.columns.slice(0, 4).map((c) => {
                  const count = D.orders.filter((o) => o.stage === c.key).length;
                  return (
                    <div key={c.key} style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <Badge tone={c.tone} dot size="sm">{c.key}</Badge>
                      </div>
                      <span style={{ fontFamily: 'var(--font-mono)', fontWeight: 600, color: 'var(--text-primary)' }} className="tabular">{count}</span>
                    </div>
                  );
                })}
              </div>
            </Card>
            <Card title="Today's crew">
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <AvatarGroup names={['Max Bowen', 'Dana Ruiz', 'Priya Shah', 'Lee Ortiz', 'Sam Kade']} max={4} />
                <span style={{ font: 'var(--font-caption)', color: 'var(--text-tertiary)' }}>5 operators</span>
              </div>
            </Card>
          </div>
        </div>
      </div>
    );
  }

  window.Dashboard = Dashboard;
})();
