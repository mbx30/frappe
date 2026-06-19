/* Orders screen — filterable table */
(function () {
  const { Card, Badge, Button, Input, Tabs, Tag, IconButton } = window.FrappeDesignSystem_75694f;
  const { Ic, PageHeader } = window.FK;
  const D = window.FrappeData;

  function Orders({ onOpenOrder, onNewOrder }) {
    const [filter, setFilter] = React.useState('All');
    const [q, setQ] = React.useState('');

    const filtered = D.orders.filter((o) => {
      const matchQ = !q || (o.customer + o.job + o.id).toLowerCase().includes(q.toLowerCase());
      const matchF =
        filter === 'All' ? true :
        filter === 'Open' ? o.status !== 'shipped' :
        filter === 'Rush' ? o.rush :
        filter === 'Overdue' ? o.status === 'overdue' :
        filter === 'Shipped' ? o.status === 'shipped' : true;
      return matchQ && matchF;
    });

    const th = (label, align) => (
      <th style={{ textAlign: align || 'left', font: 'var(--font-caption)', fontWeight: 600, textTransform: 'uppercase', letterSpacing: 'var(--tracking-caps)', color: 'var(--text-tertiary)', padding: '9px 16px', borderBottom: '1px solid var(--border-subtle)', whiteSpace: 'nowrap' }}>{label}</th>
    );
    const td = (children, extra) => (
      <td style={{ padding: '12px 16px', borderBottom: '1px solid var(--border-subtle)', font: 'var(--font-body)', color: 'var(--text-secondary)', ...extra }}>{children}</td>
    );

    return (
      <div>
        <PageHeader
          title="Orders"
          subtitle={`${D.orders.length} total · ${D.orders.filter(o=>o.status!=='shipped').length} open`}
          actions={<Button variant="primary" iconLeft={<Ic n="Plus" size={15} />} onClick={onNewOrder}>New order</Button>}
        />

        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '12px', marginBottom: '14px', flexWrap: 'wrap' }}>
          <Tabs variant="pill" value={filter} onChange={setFilter} tabs={['All', 'Open', 'Rush', 'Overdue', 'Shipped']} />
          <div style={{ width: '240px' }}>
            <Input placeholder="Search orders…" value={q} onChange={(e) => setQ(e.target.value)} iconLeft={<Ic n="Search" size={14} />} size="sm" />
          </div>
        </div>

        <Card padding="none">
          <table style={{ width: '100%', borderCollapse: 'collapse' }}>
            <thead>
              <tr>
                {th('Order')}{th('Customer')}{th('Job')}{th('Qty', 'right')}{th('Due')}{th('Status')}{th('Total', 'right')}{th('')}
              </tr>
            </thead>
            <tbody>
              {filtered.map((o) => {
                const b = D.statusBadge[o.status];
                return (
                  <tr key={o.id} onClick={() => onOpenOrder(o)} style={{ cursor: 'pointer' }}
                    onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--surface-hover)')}
                    onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}>
                    {td(<span style={{ display: 'flex', alignItems: 'center', gap: '7px' }}><span style={{ fontFamily: 'var(--font-mono)', fontWeight: 500, color: 'var(--text-primary)' }} className="tabular">#{o.id}</span>{o.rush && <Badge tone="brand" size="sm">Rush</Badge>}</span>)}
                    {td(<span style={{ font: 'var(--font-body-strong)', color: 'var(--text-primary)' }}>{o.customer}</span>)}
                    {td(<span style={{ display: 'inline-block', maxWidth: '200px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{o.job}</span>)}
                    {td(<span className="tabular" style={{ fontFamily: 'var(--font-mono)' }}>{o.qty.toLocaleString()}</span>, { textAlign: 'right', color: 'var(--text-primary)' })}
                    {td(<span style={{ color: o.status === 'overdue' ? 'var(--danger-text)' : 'var(--text-secondary)', fontWeight: o.status === 'overdue' ? 600 : 400 }}>{o.due}</span>)}
                    {td(<Badge tone={b.tone} dot size="sm">{b.label}</Badge>)}
                    {td(<span style={{ fontFamily: 'var(--font-mono)', fontWeight: 500, color: 'var(--text-primary)' }} className="tabular">{D.money(o.total)}</span>, { textAlign: 'right' })}
                    {td(<span style={{ color: 'var(--text-tertiary)' }}><Ic n="ChevronRight" size={16} /></span>, { width: '32px' })}
                  </tr>
                );
              })}
            </tbody>
          </table>
          {filtered.length === 0 && (
            <div style={{ padding: '48px', textAlign: 'center', color: 'var(--text-tertiary)', font: 'var(--font-body)' }}>No orders match that filter.</div>
          )}
        </Card>
      </div>
    );
  }

  window.Orders = Orders;
})();
