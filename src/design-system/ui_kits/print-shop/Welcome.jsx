/* Welcome / sign-in + business onboarding (roadmap #5) */
(function () {
  const { Button, Input, Select } = window.FrappeDesignSystem_75694f;
  const { Ic } = window.FK;

  function Welcome({ onEnter }) {
    const [step, setStep] = React.useState('signin'); // signin | onboard

    return (
      <div style={{ height: '100vh', width: '100vw', display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'var(--bg-base)', padding: '24px', boxSizing: 'border-box' }}>
        <div style={{ width: '380px', maxWidth: '100%' }}>
          <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', marginBottom: '24px' }}>
            <img src="../../assets/frappe-logo.svg" width="44" height="42" alt="Frappe" />
            <div style={{ marginTop: '14px', fontFamily: 'var(--font-sans)', fontWeight: 600, fontSize: 'var(--text-2xl)', lineHeight: 1.25, color: 'var(--text-primary)', letterSpacing: 'var(--tracking-tight)', textAlign: 'center', whiteSpace: 'nowrap' }}>
              {step === 'signin' ? 'Welcome to Frappe' : 'Tell us about your shop'}
            </div>
            <div style={{ marginTop: '6px', fontFamily: 'var(--font-sans)', fontWeight: 400, fontSize: 'var(--text-md)', lineHeight: 1.5, color: 'var(--text-secondary)', textAlign: 'center' }}>
              {step === 'signin' ? 'Run your print shop — quotes to shipping.' : 'We’ll tailor your workspace.'}
            </div>
          </div>

          <div style={{ background: 'var(--surface-card)', border: '1px solid var(--border-default)', borderRadius: 'var(--radius-lg)', boxShadow: 'var(--shadow-md)', padding: '22px' }}>
            {step === 'signin' ? (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                <Button variant="secondary" fullWidth iconLeft={<Ic n="Mail" size={16} />} onClick={() => setStep('onboard')}>Continue with email</Button>
                <Button variant="secondary" fullWidth onClick={() => setStep('onboard')}>
                  <span style={{ display: 'inline-flex', marginRight: '8px' }}><svg width="16" height="16"><use href="../../assets/social-icons.svg#google-sheets-icon" /></svg></span>
                  Continue with Google
                </Button>
                <div style={{ textAlign: 'center', font: 'var(--font-caption)', color: 'var(--text-tertiary)', margin: '4px 0' }}>Choose how you’d like to sign in.</div>
              </div>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
                <Input label="Shop name" placeholder="Bowen Print Co." defaultValue="Bowen Print Co." />
                <Select label="What do you print most?" options={['Business cards & stationery', 'Large format & signage', 'Wedding & events', 'Apparel & promo', 'A bit of everything']} />
                <Select label="Team size" options={['Just me', '2–5', '6–15', '16+']} />
                <Button variant="primary" fullWidth iconRight={<Ic n="ArrowRight" size={16} />} onClick={onEnter}>Open my workspace</Button>
              </div>
            )}
          </div>

          <div style={{ textAlign: 'center', marginTop: '16px', font: 'var(--font-caption)', color: 'var(--text-tertiary)' }}>
            Local-first · your data stays on this machine
          </div>
        </div>
      </div>
    );
  }

  window.Welcome = Welcome;
})();
