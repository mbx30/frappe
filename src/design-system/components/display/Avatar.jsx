import React from 'react';

const SIZES = { xs: 22, sm: 28, md: 36, lg: 48 };
const FONT = { xs: 9, sm: 11, md: 13, lg: 17 };

const PALETTE = [
  ['#ece3ff', '#6913e0'], ['#d6eeff', '#0a6ba3'], ['#d8f5e3', '#0f7035'],
  ['#fdeccc', '#8a5400'], ['#fbdedd', '#c11f1f'], ['#e3e3e9', '#3e3e47'],
];

function initials(name = '') {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (!parts.length) return '?';
  return (parts[0][0] + (parts[1] ? parts[1][0] : '')).toUpperCase();
}
function hash(s = '') {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) | 0;
  return Math.abs(h);
}

/**
 * Avatar — circular image or auto-colored initials fallback.
 */
export function Avatar({ name = '', src, size = 'md', style, ...rest }) {
  const px = SIZES[size] || SIZES.md;
  const [bg, fg] = PALETTE[hash(name) % PALETTE.length];
  return (
    <span style={{
      width: px, height: px, borderRadius: '50%', flex: 'none',
      display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
      overflow: 'hidden', background: src ? 'var(--surface-active)' : bg, color: fg,
      fontFamily: 'var(--font-sans)', fontSize: FONT[size] || 13, fontWeight: 'var(--weight-semibold)',
      userSelect: 'none', ...style,
    }} title={name} {...rest}>
      {src
        ? <img src={src} alt={name} style={{ width: '100%', height: '100%', objectFit: 'cover' }} />
        : initials(name)}
    </span>
  );
}

/**
 * AvatarGroup — overlapping stack with optional +N overflow.
 */
export function AvatarGroup({ names = [], max = 4, size = 'md' }) {
  const px = SIZES[size] || SIZES.md;
  const shown = names.slice(0, max);
  const extra = names.length - shown.length;
  return (
    <span style={{ display: 'inline-flex', alignItems: 'center' }}>
      {shown.map((n, i) => (
        <span key={i} style={{ marginLeft: i ? -px * 0.3 : 0, borderRadius: '50%', boxShadow: '0 0 0 2px var(--surface-card)' }}>
          <Avatar name={n} size={size} />
        </span>
      ))}
      {extra > 0 && (
        <span style={{
          marginLeft: -px * 0.3, width: px, height: px, borderRadius: '50%',
          background: 'var(--surface-active)', color: 'var(--text-secondary)',
          display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
          fontFamily: 'var(--font-sans)', fontSize: FONT[size] || 13, fontWeight: 600,
          boxShadow: '0 0 0 2px var(--surface-card)',
        }}>+{extra}</span>
      )}
    </span>
  );
}
