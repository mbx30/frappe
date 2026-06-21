/* Job ticket / docket — order detail view (+ art approval + fulfillment) */
(function () {
  const { useState } = React;
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

  /* ── Art Approval panel ───────────────────────────────── */
  const APPROVALS = [
    { version: 1, status: 'changes_requested', submittedAt: 'Jun 17', respondedAt: 'Jun 18', filePath: '/proofs/acme-cards-v1.pdf', staffNotes: 'First proof — please confirm bleed and font.', customerNotes: 'Logo feels too small. Can we increase by 15%?' },
    { version: 2, status: 'pending',           submittedAt: 'Jun 19', respondedAt: null,      filePath: '/proofs/acme-cards-v2.pdf', staffNotes: 'Revised — logo enlarged, font adjusted.', customerNotes: null },
  ];
  const APPROVAL_TONE  = { pending: 'warning', approved: 'success', changes_requested: 'danger' };
  const APPROVAL_LABEL = { pending: 'Awaiting response', approved: 'Approved', changes_requested: 'Changes requested' };

  function ArtApprovalPanel() {
    const [showForm, setShowForm] = useState(false);
    const pending = APPROVALS.find(a => a.status === 'pending');
    return (
      <Card title="Art approval">
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '12px' }}>
          <span style={{ font: 'var(--font-caption)', color: 'var(--text-tertiary)' }}>{APPROVALS.length} proof{APPROVALS.length !== 1 ? 's' : ''} submitted</span>
          {!pending && <Button variant="secondary" size="sm" iconLeft={<Ic n="Upload" size={13} />} onClick={() => setShowForm(v => !v)}>Submit proof</Button>}
          {pending  && <span style={{ font: 'var(--font-caption)', color: 'var(--warning-text)', display: 'flex', alignItems: 'center', gap: '5px' }}><Ic n="Clock" size={12} />Awaiting customer</span>}
        </div>

        {showForm && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '10px', padding: '12px', borderRadius: 'var(--radius-md)', background: 'var(--surface-inset)', border: '1px solid var(--border-subtle)', marginBottom: '14px' }}>
            <div style={{ font: 'var(--font-label)', color: 'var(--text-primary)' }}>Submit proof — v{APPROVALS.length + 1}</div>
            <input placeholder="File path / shared drive link" style={{ width: '100%', height: '32px', boxSizing: 'border-box', padding: '0 10px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-default)', background: 'var(--surface-card)', color: 'var(--text-primary)', font: 'var(--font-body)', outline: 'none' }} />
            <textarea rows={2} placeholder="Notes for customer…" style={{ width: '100%', boxSizing: 'border-box', padding: '8px 10px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-default)', background: 'var(--surface-card)', color: 'var(--text-primary)', font: 'var(--font-body)', outline: 'none', resize: 'vertical' }} />
            <div style={{ display: 'flex', gap: '8px', justifyContent: 'flex-end' }}>
              <Button variant="secondary" size="sm" onClick={() => setShowForm(false)}>Cancel</Button>
              <Button variant="primary" size="sm">Submit</Button>
            </div>
          </div>
        )}

        <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
          {[...APPROVALS].reverse().map(a => (
            <div key={a.version} style={{ padding: '11px 12px', borderRadius: 'var(--radius-md)', border: '1px solid var(--border-subtle)', background: a.status === 'changes_requested' ? 'var(--danger-subtle)' : a.status === 'pending' ? 'var(--warning-subtle)' : 'var(--surface-inset)' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '7px' }}>
                <span style={{ font: 'var(--font-label)', fontFamily: 'var(--font-mono)', color: 'var(--text-secondary)' }}>v{a.version}</span>
                <Badge tone={APPROVAL_TONE[a.status]} size="sm">{APPROVAL_LABEL[a.status]}</Badge>
                <span style={{ marginLeft: 'auto', font: 'var(--font-caption)', color: 'var(--text-tertiary)' }}>{a.submittedAt}</span>
              </div>
              {a.filePath && (
                <div style={{ display: 'flex', alignItems: 'center', gap: '6px', font: 'var(--font-caption)', color: 'var(--text-secondary)', marginBottom: '5px' }}>
                  <Ic n="FileText" size={12} /><span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{a.filePath}</span>
                </div>
              )}
              {a.staffNotes && <div style={{ font: 'var(--font-caption)', color: 'var(--text-secondary)', marginBottom: a.customerNotes ? '5px' : 0 }}><b>Staff:</b> {a.staffNotes}</div>}
              {a.customerNotes && <div style={{ font: 'var(--font-caption)', color: 'var(--text-primary)' }}><b>Customer:</b> {a.customerNotes}</div>}
            </div>
          ))}
        </div>
      </Card>
    );
  }

  /* ── Fulfillment panel ──────────────────────────────────── */
  function FulfillmentPanel() {
    const [method, setMethod]   = useState('ship');
    const [carrier, setCarrier] = useState('UPS');
    const [tracking, setTracking] = useState('1Z9999W99999999999');
    const [shippedAt, setShippedAt] = useState('2025-06-19');
    const [ready, setReady]     = useState(false);
    const [dirty, setDirty]     = useState(false);

    const upd = (fn) => { fn(); setDirty(true); };

    return (
      <Card title="Fulfillment">
        <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
          <div>
            <div style={{ font: 'var(--font-caption)', fontWeight: 600, color: 'var(--text-tertiary)', marginBottom: '5px' }}>Method</div>
            <div style={{ display: 'flex', gap: '6px' }}>
              {['pickup', 'ship', 'delivery'].map(m => (
                <button key={m} onClick={() => upd(() => setMethod(m))}
                  style={{ padding: '5px 12px', borderRadius: 'var(--radius-sm)', border: `1px solid ${method === m ? 'var(--brand)' : 'var(--border-default)'}`, background: method === m ? 'var(--brand-subtle)' : 'var(--surface-card)', color: method === m ? 'var(--brand-text)' : 'var(--text-secondary)', font: 'var(--font-label)', cursor: 'pointer' }}>
                  {m === 'pickup' ? 'Pickup' : m === 'ship' ? 'Ship' : 'Delivery'}
                </button>
              ))}
            </div>
          </div>

          {method === 'pickup' && (
            <button onClick={() => upd(() => setReady(v => !v))}
              style={{ display: 'flex', alignItems: 'center', gap: '9px', padding: '10px 12px', borderRadius: 'var(--radius-md)', border: `1px solid ${ready ? 'var(--success)' : 'var(--border-default)'}`, background: ready ? 'var(--success-subtle)' : 'var(--surface-inset)', cursor: 'pointer', width: '100%' }}>
              <span style={{ color: ready ? 'var(--success)' : 'var(--text-disabled)' }}><Ic n={ready ? 'CheckCircle2' : 'Circle'} size={16} /></span>
              <span style={{ font: 'var(--font-body-strong)', color: ready ? 'var(--success-text)' : 'var(--text-primary)' }}>{ready ? 'Ready for pickup' : 'Mark ready for pickup'}</span>
            </button>
          )}

          {(method === 'ship' || method === 'delivery') && (
            <>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px' }}>
                <div>
                  <div style={{ font: 'var(--font-caption)', fontWeight: 600, color: 'var(--text-tertiary)', marginBottom: '4px' }}>Carrier</div>
                  <input value={carrier} onChange={e => upd(() => setCarrier(e.target.value))} placeholder="UPS, FedEx, USPS…"
                    style={{ width: '100%', height: '32px', boxSizing: 'border-box', padding: '0 10px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-default)', background: 'var(--surface-card)', color: 'var(--text-primary)', font: 'var(--font-body)', outline: 'none' }} />
                </div>
                <div>
                  <div style={{ font: 'var(--font-caption)', fontWeight: 600, color: 'var(--text-tertiary)', marginBottom: '4px' }}>Shipped date</div>
                  <input type="date" value={shippedAt} onChange={e => upd(() => setShippedAt(e.target.value))}
                    style={{ width: '100%', height: '32px', boxSizing: 'border-box', padding: '0 10px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-default)', background: 'var(--surface-card)', color: 'var(--text-primary)', font: 'var(--font-body)', outline: 'none' }} />
                </div>
              </div>
              <div>
                <div style={{ font: 'var(--font-caption)', fontWeight: 600, color: 'var(--text-tertiary)', marginBottom: '4px' }}>Tracking number</div>
                <input value={tracking} onChange={e => upd(() => setTracking(e.target.value))} placeholder="Tracking number"
                  style={{ width: '100%', height: '32px', boxSizing: 'border-box', padding: '0 10px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-default)', background: 'var(--surface-card)', color: 'var(--text-primary)', fontFamily: 'var(--font-mono)', font: 'var(--font-body)', outline: 'none' }} />
              </div>
              {tracking && <div style={{ display: 'flex', alignItems: 'center', gap: '6px', font: 'var(--font-caption)', color: 'var(--success-text)', padding: '7px 10px', borderRadius: 'var(--radius-sm)', background: 'var(--success-subtle)' }}>
                <Ic n="Truck" size={13} />Shipped via {carrier || 'carrier'} · {tracking}
              </div>}
            </>
          )}

          {dirty && (
            <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
              <Button variant="primary" size="sm" onClick={() => setDirty(false)}>Save</Button>
            </div>
          )}
        </div>
      </Card>
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
            <ArtApprovalPanel />
            <FulfillmentPanel />
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
