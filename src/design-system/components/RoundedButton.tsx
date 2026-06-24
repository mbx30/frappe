import React from 'react'

type RoundedButtonVariant = 'primary' | 'secondary' | 'tertiary'

interface RoundedButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: RoundedButtonVariant
  size?: 'sm' | 'md' | 'lg'
  fullWidth?: boolean
  loading?: boolean
  iconLeft?: React.ReactNode
  iconRight?: React.ReactNode
}

const SIZES: Record<'sm' | 'md' | 'lg', { height: string; padding: string; font: string }> = {
  sm: { height: '32px', padding: '0 14px', font: '13px' },
  md: { height: '40px', padding: '0 18px', font: '14px' },
  lg: { height: '48px', padding: '0 22px', font: '15px' },
}

const VARIANTS: Record<RoundedButtonVariant, { background: string; color: string; border: string }> = {
  primary: {
    background: 'var(--brand, #2563eb)',
    color: 'var(--text-on-brand, #ffffff)',
    border: '1px solid transparent',
  },
  secondary: {
    background: 'var(--surface-card, #ffffff)',
    color: 'var(--text-primary, #111827)',
    border: '1px solid var(--border-default, #d1d5db)',
  },
  tertiary: {
    background: 'transparent',
    color: 'var(--brand, #2563eb)',
    border: '1px solid transparent',
  },
}

/**
 * Rounded-rectangle button. Mirrors the design-system `Button` but is
 * visually distinct: heavier corner radius (12 px instead of 6 px), a
 * secondary "tertiary" variant for inline links, and optional
 * `loading` / `iconLeft` / `iconRight` slots.
 */
export const RoundedButton = React.forwardRef<HTMLButtonElement, RoundedButtonProps>(function RoundedButton(
  {
    children,
    variant = 'primary',
    size = 'md',
    fullWidth = false,
    loading = false,
    iconLeft,
    iconRight,
    disabled,
    type = 'button',
    style,
    ...rest
  },
  ref
) {
  const sz = SIZES[size] || SIZES.md
  const v = VARIANTS[variant] || VARIANTS.primary
  const isDisabled = disabled || loading
  return (
    <button
      ref={ref}
      type={type}
      disabled={isDisabled}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: '8px',
        height: sz.height,
        padding: sz.padding,
        fontSize: sz.font,
        fontWeight: 500,
        lineHeight: 1,
        borderRadius: '12px',
        background: v.background,
        color: v.color,
        border: v.border,
        cursor: isDisabled ? 'not-allowed' : 'pointer',
        opacity: isDisabled ? 0.5 : 1,
        width: fullWidth ? '100%' : 'auto',
        transition: 'background-color 0.15s ease, transform 0.1s ease',
        userSelect: 'none',
        ...style,
      }}
      {...rest}
    >
      {loading ? <span aria-hidden="true">…</span> : iconLeft}
      {children && <span>{children}</span>}
      {!loading && iconRight}
    </button>
  )
})

export default RoundedButton
