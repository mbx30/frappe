/* Production screen — kanban board */
(function () {
  const { Card, Badge, Avatar, AvatarGroup, Tag } = window.FrappeDesignSystem_75694f;
  const { Ic, PageHeader } = window.FK;
  const D = window.FrappeData;

  function JobCard({ o, onOpen }) {
    return (
      <div onClick={() => onOpen(o)}
        style={{ background: 'var(--surface-card)', border: '1px solid var(--border-default)', borderRadius: 'var(--radius-md)', boxShadow: 'var(--shadow-xs)', padding: '11px 12px', cursor: 'pointer', transition: 'box-shadow var(--duration-fast), transform var(--duration-fast)' }}
        onMouseEnter={(e) => { e.currentTarget.style.boxShadow = 'var(--shadow-md)'; e.currentTarget.style.transform = 'translateY(-1px)'; }}
        onMouseLeave={(e) => { e.currentTarget.style.boxShadow = 'var(--shadow-xs)'; e.currentTarget.style.transform = 'none'; }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '7px' }}>
          <span style={{ fontFamily: 'var(--font-mono)', fontWeight: 600, fontSize: 'var(--text-sm)', color: 'var(--text-primary)' }} className="tabular">#{o.id}</span>
          {o.rush && <Badge tone="brand" size="sm">Rush</Badge>}
        </div>
        <div style={{ font: 'var(--font-body-strong)', color: 'var(--text-primary)', marginBottom: '3px', lineHeight: 1.3 }}>{o.job}</div>
        <div style={{ font: 'var(--font-caption)', color: 'var(--text-tertiary)', marginBottom: '10px' }}>{o.customer} · {o.qty.toLocaleString()} · {o.stock}</div>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <span style={{ display: 'flex', alignItems: 'center', gap: '5px', font: 'var(--font-caption)', color: o.status === 'overdue' ? 'var(--danger-text)' : 'var(--text-secondary)' }}>
            <Ic n="Clock" size={12} />{o.due}
          </span>
          {o.assignees.length > 0 ? <AvatarGroup names={o.assignees} max={2} size="xs" /> : <span style={{ font: 'var(--font-caption)', color: 'var(--text-disabled)' }}>Unassigned</span>}
        </div>
      </div>
    );
  }

  function Production({ onOpenOrder }) {
    return (
      <div>
        <PageHeader
          title="Production"
          subtitle="Drag jobs across the floor — Queued through Shipped"
        />
        <div style={{ display: 'grid', gridTemplateColumns: `repeat(${D.columns.length}, 1fr)`, gap: '12px', alignItems: 'start' }}>
          {D.columns.map((c) => {
            const jobs = D.orders.filter((o) => o.stage === c.key);
            return (
              <div key={c.key} style={{ background: 'var(--bg-subtle)', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-lg)', padding: '10px', minHeight: '120px' }}>
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '2px 4px 10px' }}>
                  <Badge tone={c.tone} dot size="sm">{c.key}</Badge>
                  <span style={{ fontFamily: 'var(--font-mono)', fontWeight: 600, fontSize: 'var(--text-xs)', color: 'var(--text-tertiary)' }} className="tabular">{jobs.length}</span>
                </div>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '9px' }}>
                  {jobs.map((o) => <JobCard key={o.id} o={o} onOpen={onOpenOrder} />)}
                  {jobs.length === 0 && <div style={{ padding: '16px 8px', textAlign: 'center', font: 'var(--font-caption)', color: 'var(--text-disabled)' }}>Empty</div>}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    );
  }

  window.Production = Production;
})();
