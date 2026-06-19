/* Job ticket / docket — order detail view */
(function () {
  const { Card, Badge, Button, Avatar, AvatarGroup, Tag, IconButton } = window.FrappeDesignSystem_75694f;
  const Tooltip = window.FrappeDesignSystem_75694f.Tooltip || (({ children }) => children);
  const { Ic, Eyebrow } = window.FK;
  const D = window.FrappeData;

  function Row({ label, children }) {
    return (
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '9px 0', borderBottom: '1px solid var(--border-subtle)' }}>
        <span style={{ font: 'var(--font-body)', color: 'var(--text-tertiary)' }}>{label}</span>
        <span style={{ font: 'var(--font-body-strong)', color: 'var(--text-primary)' }}>{children}</span>
      </div>
    );
  }

  function Step({ label, tone, done, current }) {
    return (
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
        <span style={{ width: '18px', height: '18px', borderRadius: '999px', flexShrink: 0, display: 'flex', alignItems: 'center', justifyContent: 'center',
          background: current ? 'var(--brand)' : done ? 'var(--success)' : 'var(--surface-inset)',
          border: current || done ? 'none' : '1.5px solid var(--border-strong)', color: '#fff' }}>
          {done && !current ? <Ic n="Check" size={11} /> : current ? <span style={{ width: '6px', height: '6px', borderRadius: '999px', background: '#fff' }} /> : null}
        </span>
        <span style={{ font: current ? 'var(--font-body-strong)' : 'var(--font-body)', color: current ? 'var(--text-primary)' : done ? 'var(--text-secondary)' : 'var(--text-tertiary)' }}>{label}</span>
      </div>
    );
  }

  function JobTicket({ order, onBack }) {
    const o = order;
    const b = D.statusBadge[o.status];
    const stages = ['Queued', 'Prepress', 'On press', 'Bindery', 'Shipped'];
    const curIdx = stages.indexOf(o.stage);

    return (
      <div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '18px' }}>
          <Button variant="ghost" size="sm" iconLeft={<Ic n="ArrowLeft" size={15} />} onClick={onBack}>Orders</Button>
          <span style={{ color: 'var(--text-disabled)' }}>/</span>
          <span style={{ fontFamily: 'var(--font-mono)', fontWeight: 600, color: 'var(--text-secondary)' }} className="tabular">#{o.id}</span>
        </div>

        <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: '16px', marginBottom: '22px' }}>
          <div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '6px' }}>
              <h1 style={{ margin: 0, font: 'var(--font-h2)', color: 'var(--text-primary)', letterSpacing: 'var(--tracking-tight)' }}>{o.job}</h1>
              <Badge tone={b.tone} dot>{b.label}</Badge>
              {o.rush && <Badge tone="brand">Rush</Badge>}
            </div>
            <div style={{ font: 'var(--font-body)', color: 'var(--text-secondary)' }}>{o.customer} · {o.contact}</div>
          </div>
          <div style={{ display: 'flex', gap: '8px', flexShrink: 0 }}>
            <Tooltip label="Print job ticket"><IconButton icon={<Ic n="Printer" size={16} />} label="Print" variant="secondary" /></Tooltip>
            <Button variant="secondary" iconLeft={<Ic n="Image" size={15} />}>View proof</Button>
            <Button variant="primary" iconLeft={<Ic n="Check" size={15} />}>Advance stage</Button>
          </div>
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: '1.5fr 1fr', gap: '16px', alignItems: 'start' }}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
            <Card title="Specifications">
              <Row label="Quantity"><span className="tabular" style={{ fontFamily: 'var(--font-mono)' }}>{o.qty.toLocaleString()} units</span></Row>
              <Row label="Stock">{o.stock}</Row>
              <Row label="Finishing">Trim · Round corners</Row>
              <Row label="Proof">Customer approval required</Row>
              <div style={{ paddingTop: '9px' }}><Row label="Due"><span style={{ color: o.status === 'overdue' ? 'var(--danger-text)' : 'var(--text-primary)' }}>{o.due}</span></Row></div>
            </Card>

            <Card title="Pricing">
              <Row label="Printing"><span className="tabular" style={{ fontFamily: 'var(--font-mono)' }}>{D.money(o.total * 0.78)}</span></Row>
              <Row label="Finishing"><span className="tabular" style={{ fontFamily: 'var(--font-mono)' }}>{D.money(o.total * 0.14)}</span></Row>
              <Row label="Rush surcharge"><span className="tabular" style={{ fontFamily: 'var(--font-mono)' }}>{o.rush ? D.money(o.total * 0.08) : '$0.00'}</span></Row>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', paddingTop: '12px' }}>
                <span style={{ font: 'var(--font-title)', color: 'var(--text-primary)' }}>Total</span>
                <span style={{ font: 'var(--font-h3)', fontFamily: 'var(--font-mono)', color: 'var(--text-primary)' }} className="tabular">{D.money(o.total)}</span>
              </div>
            </Card>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
            <Card title="Production stage">
              <div style={{ display: 'flex', flexDirection: 'column', gap: '13px' }}>
                {stages.map((s, i) => <Step key={s} label={s} done={i <= curIdx} current={i === curIdx} />)}
              </div>
            </Card>
            <Card title="Assigned">
              {o.assignees.length > 0 ? (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
                  {o.assignees.map((n) => (
                    <div key={n} style={{ display: 'flex', alignItems: 'center', gap: '9px' }}>
                      <Avatar name={n} size="sm" />
                      <span style={{ font: 'var(--font-body)', color: 'var(--text-primary)' }}>{n}</span>
                    </div>
                  ))}
                </div>
              ) : <div style={{ font: 'var(--font-body)', color: 'var(--text-tertiary)' }}>No operator assigned yet.</div>}
            </Card>
          </div>
        </div>
      </div>
    );
  }

  window.JobTicket = JobTicket;
})();
