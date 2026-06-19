/* Invoicing screen */
(function () {
  const { Card, Badge, Button, Tabs, Input } = window.FrappeDesignSystem_75694f;
  const { Ic, Kpi, PageHeader } = window.FK;
  const D = window.FrappeData;

  function Invoicing() {
    const [filter, setFilter] = React.useState('All');
    const list = D.invoices.filter((i) =>
      filter === 'All' ? true :
      filter === 'Unpaid' ? (i.status === 'sent' || i.status === 'overdue' || i.status === 'deposit') :
      filter === 'Paid' ? i.status === 'paid' :
      filter === 'Overdue' ? i.status === 'overdue' :
      filter === 'Draft' ? i.status === 'draft' : true
    );

    const th = (label, align) => (
      <th style={{ textAlign: align || 'left', font: 'var(--font-caption)', fontWeight: 600, textTransform: 'uppercase', letterSpacing: 'var(--tracking-caps)', color: 'var(--text-tertiary)', padding: '9px 16px', borderBottom: '1px solid var(--border-subtle)' }}>{label}</th>
    );

    return (
      <div>
        <PageHeader
          title="Invoicing"
          subtitle="Deposits, balances and payments"
          actions={<Button variant="primary" iconLeft={<Ic n="Plus" size={15} />}>New invoice</Button>}
        />

        <div style={{ display: 'flex', gap: '14px', marginBottom: '22px' }}>
          <Kpi label="Outstanding" value="$583" delta="3 invoices" deltaTone="neutral" icon="Receipt" />
          <Kpi label="Overdue" value="$159" delta="1 invoice · 9d" deltaTone="danger" icon="AlertCircle" />
          <Kpi label="Deposits held" value="$270" delta="Pine & Oak" deltaTone="neutral" icon="PiggyBank" />
          <Kpi label="Paid (MTD)" value="$1.8k" delta="+12% vs May" icon="CheckCircle2" />
        </div>

        <div style={{ marginBottom: '14px' }}>
          <Tabs variant="pill" value={filter} onChange={setFilter} tabs={['All', 'Unpaid', 'Paid', 'Overdue', 'Draft']} />
        </div>

        <Card padding="none">
          <table style={{ width: '100%', borderCollapse: 'collapse' }}>
            <thead>
              <tr>{th('Invoice')}{th('Customer')}{th('Order')}{th('Date')}{th('Deposit', 'right')}{th('Amount', 'right')}{th('Status')}{th('', 'right')}</tr>
            </thead>
            <tbody>
              {list.map((inv) => {
                const b = D.invoiceBadge[inv.status];
                const td = (children, extra) => (<td style={{ padding: '12px 16px', borderBottom: '1px solid var(--border-subtle)', font: 'var(--font-body)', color: 'var(--text-secondary)', ...extra }}>{children}</td>);
                return (
                  <tr key={inv.id} onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--surface-hover)')} onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}>
                    {td(<span style={{ fontFamily: 'var(--font-mono)', fontWeight: 500, color: 'var(--text-primary)' }} className="tabular">{inv.id}</span>)}
                    {td(<span style={{ font: 'var(--font-body-strong)', color: 'var(--text-primary)' }}>{inv.customer}</span>)}
                    {td(<span style={{ fontFamily: 'var(--font-mono)' }} className="tabular">#{inv.order}</span>)}
                    {td(inv.date)}
                    {td(<span style={{ fontFamily: 'var(--font-mono)', color: inv.deposit ? 'var(--text-primary)' : 'var(--text-disabled)' }} className="tabular">{inv.deposit ? D.money(inv.deposit) : '—'}</span>, { textAlign: 'right' })}
                    {td(<span style={{ fontFamily: 'var(--font-mono)', fontWeight: 500, color: 'var(--text-primary)' }} className="tabular">{D.money(inv.amount)}</span>, { textAlign: 'right' })}
                    {td(<Badge tone={b.tone} dot size="sm">{b.label}</Badge>)}
                    {td(inv.status === 'draft'
                      ? <Button variant="subtle" size="sm">Send</Button>
                      : inv.status === 'paid'
                        ? <span style={{ color: 'var(--text-disabled)', font: 'var(--font-caption)' }}>Paid</span>
                        : <Button variant="secondary" size="sm">Record payment</Button>,
                      { textAlign: 'right' })}
                  </tr>
                );
              })}
            </tbody>
          </table>
        </Card>
      </div>
    );
  }

  window.Invoicing = Invoicing;
})();
