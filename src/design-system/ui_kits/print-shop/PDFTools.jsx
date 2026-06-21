/* PDF Tools — preflight, inspector, color conversion, PDF/X wizard, certified version */
(function () {
  const { useState } = React;
  const { Badge, Button, Card, Select, Input } = window.FrappeDesignSystem_75694f;
  const { Ic } = window.FK;

  const SAMPLE_JOBS = [
    { id: 1, name: 'Acme-Cards-v3.pdf',       pages: 2, size: '4.8 MB',  hasIssues: true  },
    { id: 2, name: 'NorthwindBanner-FINAL.pdf', pages: 1, size: '18.2 MB', hasIssues: false },
    { id: 3, name: 'LumenBrochure-R2.pdf',      pages: 6, size: '12.7 MB', hasIssues: true  },
  ];

  const PREFLIGHT_CHECKS = [
    { label: 'Bleed',        detail: '3 mm bleed on all sides',               status: 'pass'    },
    { label: 'Color mode',   detail: 'CMYK throughout',                        status: 'pass'    },
    { label: 'Fonts',        detail: 'All fonts embedded',                     status: 'pass'    },
    { label: 'Resolution',   detail: '2 images below 300 DPI (220, 254 DPI)',  status: 'warning' },
    { label: 'Transparency', detail: 'Flattened transparency on 1 page',       status: 'warning' },
    { label: 'Spot colors',  detail: '1 unresolved spot: PANTONE 485 C',       status: 'fail'    },
    { label: 'Overprint',    detail: 'Black overprint set correctly',           status: 'pass'    },
    { label: 'File size',    detail: 'Within 50 MB limit',                     status: 'pass'    },
  ];

  const CHECK_TONE = { pass: 'success', warning: 'warning', fail: 'danger' };
  const CHECK_ICON = { pass: 'CheckCircle2', warning: 'AlertTriangle', fail: 'XCircle' };
  const CHECK_COLOR = { pass: 'var(--success)', warning: 'var(--warning)', fail: 'var(--danger)' };
  const CHECK_BG = { pass: 'transparent', warning: 'var(--warning-subtle)', fail: 'var(--danger-subtle)' };

  /* ── Preflight ──────────────────────────────────────────── */
  function PreflightTab({ job }) {
    const issues = PREFLIGHT_CHECKS.filter(c => c.status !== 'pass');
    const failures = PREFLIGHT_CHECKS.filter(c => c.status === 'fail');
    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: '18px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px', padding: '13px 15px', borderRadius: 'var(--radius-md)', background: failures.length ? 'var(--danger-subtle)' : 'var(--warning-subtle)', border: `1px solid ${failures.length ? 'var(--danger)' : 'var(--warning)'}`, color: failures.length ? 'var(--danger-text)' : 'var(--warning-text)' }}>
          <Ic n={failures.length ? 'XCircle' : 'AlertTriangle'} size={18} />
          <div>
            <div style={{ font: 'var(--font-body-strong)' }}>{failures.length ? `${failures.length} error${failures.length > 1 ? 's' : ''}` : ''}{failures.length && issues.length > failures.length ? ', ' : ''}{issues.length - failures.length > 0 ? `${issues.length - failures.length} warning${issues.length - failures.length > 1 ? 's' : ''}` : ''}</div>
            <div style={{ font: 'var(--font-caption)', marginTop: '2px' }}>Resolve errors before sending to press.</div>
          </div>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
          {PREFLIGHT_CHECKS.map(c => (
            <div key={c.label} style={{ display: 'flex', alignItems: 'center', gap: '12px', padding: '10px 12px', borderRadius: 'var(--radius-sm)', background: CHECK_BG[c.status] }}>
              <span style={{ color: CHECK_COLOR[c.status], flexShrink: 0 }}><Ic n={CHECK_ICON[c.status]} size={16} /></span>
              <span style={{ font: 'var(--font-body-strong)', width: '130px', flexShrink: 0 }}>{c.label}</span>
              <span style={{ font: 'var(--font-body)', color: 'var(--text-secondary)', flex: 1 }}>{c.detail}</span>
              <Badge tone={CHECK_TONE[c.status]} size="sm">{c.status === 'pass' ? 'Pass' : c.status === 'warning' ? 'Warning' : 'Fail'}</Badge>
            </div>
          ))}
        </div>

        <div style={{ display: 'flex', gap: '8px' }}>
          <Button variant="primary" iconLeft={<Ic n="RefreshCw" size={14} />}>Re-run preflight</Button>
          <Button variant="secondary" iconLeft={<Ic n="FileDown" size={14} />}>Export report</Button>
        </div>
      </div>
    );
  }

  /* ── Inspector ──────────────────────────────────────────── */
  function InspectorTab({ job }) {
    const rows = [
      ['Filename',    job.name,                       false],
      ['Pages',       String(job.pages),               true ],
      ['File size',   job.size,                        true ],
      ['PDF version', '1.7',                           true ],
      ['Color spaces','CMYK, sRGB',                    false],
      ['Page size',   '3.5 × 2 in (252 × 144 pt)',     true ],
      ['Bleed box',   '3.625 × 2.125 in',              true ],
      ['Trim box',    '3.5 × 2 in',                    true ],
      ['Creator',     'Adobe Illustrator 28.1',        false],
      ['Fonts',       '3 embedded, 0 missing',         true ],
      ['Images',      '4 total — 2 below 300 DPI',     false],
    ];
    return (
      <div>
        {rows.map(([k, v, mono]) => (
          <div key={k} style={{ display: 'flex', gap: '16px', padding: '9px 12px', borderBottom: '1px solid var(--border-subtle)' }}>
            <span style={{ font: 'var(--font-label)', color: 'var(--text-tertiary)', width: '130px', flexShrink: 0 }}>{k}</span>
            <span style={{ font: 'var(--font-body)', color: 'var(--text-primary)', fontFamily: mono ? 'var(--font-mono)' : undefined }}>{v}</span>
          </div>
        ))}
      </div>
    );
  }

  /* ── Color conversion ───────────────────────────────────── */
  function ColorTab() {
    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: '18px', maxWidth: '500px' }}>
        <div style={{ font: 'var(--font-body)', color: 'var(--text-secondary)' }}>Convert spot colors and ICC profiles to press-ready CMYK.</div>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
          <Select label="Source profile"    options={['Document ICC', 'sRGB IEC61966', 'Adobe RGB (1998)', 'FOGRA39']} />
          <Select label="Target profile"   options={['Coated FOGRA39', 'FOGRA51', 'US Web Coated SWOP', 'GRACoL 2006']} />
          <Select label="Rendering intent" options={['Relative colorimetric', 'Perceptual', 'Saturation', 'Absolute colorimetric']} />
          <Select label="Black point"      options={['Compensate', 'None']} />
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <input type="checkbox" id="flattenSpot" defaultChecked style={{ accentColor: 'var(--brand)', width: '14px', height: '14px' }} />
          <label htmlFor="flattenSpot" style={{ font: 'var(--font-body)', color: 'var(--text-primary)', cursor: 'pointer' }}>Convert spot colors to process CMYK</label>
        </div>
        <div style={{ display: 'flex', gap: '8px' }}>
          <Button variant="primary" iconLeft={<Ic n="Palette" size={14} />}>Convert colors</Button>
          <Button variant="secondary">Preview</Button>
        </div>
      </div>
    );
  }

  /* ── PDF/X wizard ───────────────────────────────────────── */
  function PdfXTab() {
    const steps = ['Embed all fonts', 'Flatten transparency', 'Remove JavaScript', 'Set output intent', 'Add document info'];
    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: '18px', maxWidth: '500px' }}>
        <div style={{ font: 'var(--font-body)', color: 'var(--text-secondary)' }}>Convert to a PDF/X standard for reliable press delivery.</div>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
          <Select label="Target standard" options={['PDF/X-4 (recommended)', 'PDF/X-1a:2001', 'PDF/X-3:2003']} />
          <Select label="Output intent"   options={['Coated FOGRA39 (ISO 12647-2)', 'GRACoL 2006', 'FOGRA51']} />
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', padding: '14px', borderRadius: 'var(--radius-md)', background: 'var(--surface-inset)', border: '1px solid var(--border-subtle)' }}>
          <div style={{ font: 'var(--font-caption)', fontWeight: 600, textTransform: 'uppercase', letterSpacing: 'var(--tracking-caps)', color: 'var(--text-tertiary)', marginBottom: '4px' }}>Steps applied</div>
          {steps.map(s => (
            <div key={s} style={{ display: 'flex', alignItems: 'center', gap: '9px' }}>
              <span style={{ color: 'var(--success)' }}><Ic n="Check" size={14} /></span>
              <span style={{ font: 'var(--font-body)', color: 'var(--text-primary)' }}>{s}</span>
            </div>
          ))}
        </div>
        <div style={{ display: 'flex', gap: '8px' }}>
          <Button variant="primary" iconLeft={<Ic n="FileCheck" size={14} />}>Create PDF/X</Button>
          <Button variant="secondary">Learn more</Button>
        </div>
      </div>
    );
  }

  /* ── Certified version ──────────────────────────────────── */
  function CertifiedTab() {
    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: '18px', maxWidth: '500px' }}>
        <div style={{ font: 'var(--font-body)', color: 'var(--text-secondary)' }}>Generate a certified, press-ready version with a full audit trail.</div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px', padding: '14px 16px', borderRadius: 'var(--radius-md)', background: 'var(--warning-subtle)', border: '1px solid var(--warning)' }}>
          <span style={{ color: 'var(--warning)' }}><Ic n="ShieldAlert" size={20} /></span>
          <div>
            <div style={{ font: 'var(--font-body-strong)', color: 'var(--warning-text)' }}>Not yet certified</div>
            <div style={{ font: 'var(--font-caption)', color: 'var(--text-secondary)', marginTop: '3px' }}>Resolve all preflight errors before generating a certified version.</div>
          </div>
        </div>
        <Select label="Certification profile" options={['Bowen Print Co. — Standard', 'Bowen Print Co. — Wide Format', 'ISO 12647-2 Sheetfed Offset']} />
        <Input label="Output filename" value="Acme-Cards-v3-CERTIFIED.pdf" />
        <div style={{ display: 'flex', gap: '8px' }}>
          <Button variant="primary" disabled iconLeft={<Ic n="ShieldCheck" size={14} />}>Generate certified PDF</Button>
          <Button variant="secondary" iconLeft={<Ic n="Clock" size={14} />}>View previous versions</Button>
        </div>
      </div>
    );
  }

  /* ── Tabs config ────────────────────────────────────────── */
  const TABS = [
    { id: 'preflight',  label: 'Preflight',  icon: 'ShieldCheck' },
    { id: 'inspector',  label: 'Inspector',  icon: 'Search'      },
    { id: 'color',      label: 'Color',      icon: 'Palette'     },
    { id: 'pdfx',       label: 'PDF/X',      icon: 'FileCheck'   },
    { id: 'certified',  label: 'Certified',  icon: 'Award'       },
  ];

  /* ── Root component ─────────────────────────────────────── */
  function PDFTools() {
    const [selected, setSelected] = useState(SAMPLE_JOBS[0]);
    const [tab, setTab]           = useState('preflight');

    return (
      <div style={{ display: 'flex', height: 'calc(100vh - 48px - 44px)', overflow: 'hidden', margin: '-22px -26px' }}>

        {/* Sidebar */}
        <div style={{ width: '224px', flexShrink: 0, borderRight: '1px solid var(--border-default)', background: 'var(--surface-card)', display: 'flex', flexDirection: 'column' }}>
          <div style={{ padding: '13px 12px 10px', borderBottom: '1px solid var(--border-subtle)' }}>
            <div style={{ font: 'var(--font-caption)', fontWeight: 600, textTransform: 'uppercase', letterSpacing: 'var(--tracking-caps)', color: 'var(--text-tertiary)', marginBottom: '8px' }}>Recent PDFs</div>
            <Button variant="secondary" fullWidth iconLeft={<Ic n="FolderOpen" size={14} />}>Open PDF…</Button>
          </div>
          <div style={{ flex: 1, overflowY: 'auto', padding: '6px' }}>
            {SAMPLE_JOBS.map(job => {
              const active = selected?.id === job.id;
              return (
                <button key={job.id} onClick={() => { setSelected(job); setTab('preflight'); }}
                  onMouseEnter={e => { if (!active) e.currentTarget.style.background = 'var(--surface-hover)'; }}
                  onMouseLeave={e => { if (!active) e.currentTarget.style.background = 'transparent'; }}
                  style={{ width: '100%', textAlign: 'left', padding: '9px 10px', marginBottom: '2px', borderRadius: 'var(--radius-md)', border: 'none', cursor: 'pointer', background: active ? 'var(--brand-subtle)' : 'transparent', display: 'flex', flexDirection: 'column', gap: '3px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '6px', minWidth: 0 }}>
                    <span style={{ color: active ? 'var(--brand)' : 'var(--text-tertiary)', flexShrink: 0 }}><Ic n="FileText" size={14} /></span>
                    <span style={{ font: 'var(--font-label)', color: active ? 'var(--brand-text)' : 'var(--text-primary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{job.name}</span>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '6px', paddingLeft: '20px' }}>
                    <span style={{ font: 'var(--font-caption)', color: 'var(--text-tertiary)', fontFamily: 'var(--font-mono)' }}>{job.pages}p · {job.size}</span>
                    {job.hasIssues && <span style={{ color: 'var(--warning)', display: 'inline-flex' }}><Ic n="AlertTriangle" size={11} /></span>}
                  </div>
                </button>
              );
            })}
          </div>
        </div>

        {/* Main */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden', minWidth: 0 }}>
          {/* File header bar */}
          <div style={{ padding: '11px 20px', borderBottom: '1px solid var(--border-default)', background: 'var(--surface-card)', display: 'flex', alignItems: 'center', gap: '14px', flexShrink: 0 }}>
            <span style={{ color: 'var(--brand)', flexShrink: 0 }}><Ic n="FileText" size={20} /></span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ font: 'var(--font-title)', color: 'var(--text-primary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{selected.name}</div>
              <div style={{ font: 'var(--font-caption)', color: 'var(--text-tertiary)', fontFamily: 'var(--font-mono)', marginTop: '2px' }}>{selected.pages} pages · {selected.size}</div>
            </div>
            <div style={{ display: 'flex', gap: '8px', flexShrink: 0 }}>
              <Button variant="secondary" size="sm" iconLeft={<Ic n="Eye" size={13} />}>View pages</Button>
              <Button variant="secondary" size="sm" iconLeft={<Ic n="Download" size={13} />}>Save job</Button>
            </div>
          </div>

          {/* Tab bar */}
          <div style={{ display: 'flex', gap: '0', padding: '0 20px', borderBottom: '1px solid var(--border-default)', background: 'var(--surface-card)', flexShrink: 0 }}>
            {TABS.map(t => {
              const active = tab === t.id;
              return (
                <button key={t.id} onClick={() => setTab(t.id)}
                  style={{ display: 'flex', alignItems: 'center', gap: '6px', padding: '9px 13px', border: 'none', background: 'transparent', cursor: 'pointer', font: 'var(--font-label)', color: active ? 'var(--brand-text)' : 'var(--text-secondary)', borderBottom: active ? '2px solid var(--brand)' : '2px solid transparent', marginBottom: '-1px', borderRadius: 0 }}>
                  <span style={{ color: active ? 'var(--brand)' : 'var(--text-tertiary)', display: 'inline-flex' }}><Ic n={t.icon} size={14} /></span>
                  {t.label}
                </button>
              );
            })}
          </div>

          {/* Tab content */}
          <div style={{ flex: 1, overflowY: 'auto', padding: '22px 24px' }}>
            {tab === 'preflight'  && <PreflightTab  job={selected} />}
            {tab === 'inspector'  && <InspectorTab  job={selected} />}
            {tab === 'color'      && <ColorTab />}
            {tab === 'pdfx'       && <PdfXTab />}
            {tab === 'certified'  && <CertifiedTab />}
          </div>
        </div>
      </div>
    );
  }

  window.PDFTools = PDFTools;
})();
