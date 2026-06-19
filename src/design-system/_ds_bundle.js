/* @ds-bundle: {"format":3,"namespace":"FrappeDesignSystem_75694f","components":[{"name":"Avatar","sourcePath":"components/display/Avatar.jsx"},{"name":"AvatarGroup","sourcePath":"components/display/Avatar.jsx"},{"name":"Badge","sourcePath":"components/display/Badge.jsx"},{"name":"Card","sourcePath":"components/display/Card.jsx"},{"name":"Tag","sourcePath":"components/display/Tag.jsx"},{"name":"Dialog","sourcePath":"components/feedback/Dialog.jsx"},{"name":"Toast","sourcePath":"components/feedback/Toast.jsx"},{"name":"ToastViewport","sourcePath":"components/feedback/Toast.jsx"},{"name":"Tooltip","sourcePath":"components/feedback/Tooltip.jsx"},{"name":"Button","sourcePath":"components/forms/Button.jsx"},{"name":"Checkbox","sourcePath":"components/forms/Checkbox.jsx"},{"name":"IconButton","sourcePath":"components/forms/IconButton.jsx"},{"name":"Input","sourcePath":"components/forms/Input.jsx"},{"name":"Select","sourcePath":"components/forms/Select.jsx"},{"name":"Switch","sourcePath":"components/forms/Switch.jsx"},{"name":"Tabs","sourcePath":"components/navigation/Tabs.jsx"}],"sourceHashes":{"components/display/Avatar.jsx":"8496b0ce7014","components/display/Badge.jsx":"ac17dc358108","components/display/Card.jsx":"7f070790414b","components/display/Tag.jsx":"04d646da7a81","components/feedback/Dialog.jsx":"3ea9b432eac3","components/feedback/Toast.jsx":"c9e9bbb66f80","components/feedback/Tooltip.jsx":"71e4c79f5f88","components/forms/Button.jsx":"59039d35f9ed","components/forms/Checkbox.jsx":"afcfe1acf061","components/forms/IconButton.jsx":"81f84e5a7ee4","components/forms/Input.jsx":"8a2170cc62d4","components/forms/Select.jsx":"160ebc61234f","components/forms/Switch.jsx":"00ddc7f912e6","components/navigation/Tabs.jsx":"6038b80e6c3d","ui_kits/print-shop/AppShell.jsx":"97b7cdcd637d","ui_kits/print-shop/Dashboard.jsx":"d5dcc678e269","ui_kits/print-shop/Invoicing.jsx":"a9eaecef6ca3","ui_kits/print-shop/JobTicket.jsx":"3e73f50451dc","ui_kits/print-shop/Orders.jsx":"250ea3eb8917","ui_kits/print-shop/Production.jsx":"90e0282b952b","ui_kits/print-shop/Welcome.jsx":"73f16e15dd6b","ui_kits/print-shop/data.js":"7ca79cbe744c","ui_kits/print-shop/shared.jsx":"85a9dadc0e95"},"inlinedExternals":[],"unexposedExports":[]} */

(() => {

const __ds_ns = (window.FrappeDesignSystem_75694f = window.FrappeDesignSystem_75694f || {});

const __ds_scope = {};

(__ds_ns.__errors = __ds_ns.__errors || []);

// components/display/Avatar.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const SIZES = {
  xs: 22,
  sm: 28,
  md: 36,
  lg: 48
};
const FONT = {
  xs: 9,
  sm: 11,
  md: 13,
  lg: 17
};
const PALETTE = [['#ece3ff', '#6913e0'], ['#d6eeff', '#0a6ba3'], ['#d8f5e3', '#0f7035'], ['#fdeccc', '#8a5400'], ['#fbdedd', '#c11f1f'], ['#e3e3e9', '#3e3e47']];
function initials(name = '') {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (!parts.length) return '?';
  return (parts[0][0] + (parts[1] ? parts[1][0] : '')).toUpperCase();
}
function hash(s = '') {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = h * 31 + s.charCodeAt(i) | 0;
  return Math.abs(h);
}

/**
 * Avatar — circular image or auto-colored initials fallback.
 */
function Avatar({
  name = '',
  src,
  size = 'md',
  style,
  ...rest
}) {
  const px = SIZES[size] || SIZES.md;
  const [bg, fg] = PALETTE[hash(name) % PALETTE.length];
  return /*#__PURE__*/React.createElement("span", _extends({
    style: {
      width: px,
      height: px,
      borderRadius: '50%',
      flex: 'none',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      overflow: 'hidden',
      background: src ? 'var(--surface-active)' : bg,
      color: fg,
      fontFamily: 'var(--font-sans)',
      fontSize: FONT[size] || 13,
      fontWeight: 'var(--weight-semibold)',
      userSelect: 'none',
      ...style
    },
    title: name
  }, rest), src ? /*#__PURE__*/React.createElement("img", {
    src: src,
    alt: name,
    style: {
      width: '100%',
      height: '100%',
      objectFit: 'cover'
    }
  }) : initials(name));
}

/**
 * AvatarGroup — overlapping stack with optional +N overflow.
 */
function AvatarGroup({
  names = [],
  max = 4,
  size = 'md'
}) {
  const px = SIZES[size] || SIZES.md;
  const shown = names.slice(0, max);
  const extra = names.length - shown.length;
  return /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      alignItems: 'center'
    }
  }, shown.map((n, i) => /*#__PURE__*/React.createElement("span", {
    key: i,
    style: {
      marginLeft: i ? -px * 0.3 : 0,
      borderRadius: '50%',
      boxShadow: '0 0 0 2px var(--surface-card)'
    }
  }, /*#__PURE__*/React.createElement(Avatar, {
    name: n,
    size: size
  }))), extra > 0 && /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: -px * 0.3,
      width: px,
      height: px,
      borderRadius: '50%',
      background: 'var(--surface-active)',
      color: 'var(--text-secondary)',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontFamily: 'var(--font-sans)',
      fontSize: FONT[size] || 13,
      fontWeight: 600,
      boxShadow: '0 0 0 2px var(--surface-card)'
    }
  }, "+", extra));
}
Object.assign(__ds_scope, { Avatar, AvatarGroup });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/display/Avatar.jsx", error: String((e && e.message) || e) }); }

// components/display/Badge.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const TONES = {
  neutral: {
    bg: 'var(--surface-active)',
    fg: 'var(--text-secondary)',
    dot: 'var(--text-tertiary)'
  },
  brand: {
    bg: 'var(--brand-subtle)',
    fg: 'var(--brand-text)',
    dot: 'var(--brand)'
  },
  success: {
    bg: 'var(--success-subtle)',
    fg: 'var(--success-text)',
    dot: 'var(--success)'
  },
  warning: {
    bg: 'var(--warning-subtle)',
    fg: 'var(--warning-text)',
    dot: 'var(--warning)'
  },
  danger: {
    bg: 'var(--danger-subtle)',
    fg: 'var(--danger-text)',
    dot: 'var(--danger)'
  },
  info: {
    bg: 'var(--info-subtle)',
    fg: 'var(--info-text)',
    dot: 'var(--info)'
  }
};
const SIZES = {
  sm: {
    font: 'var(--text-2xs)',
    pad: '2px 7px',
    dot: 5
  },
  md: {
    font: 'var(--text-xs)',
    pad: '3px 9px',
    dot: 6
  }
};

/**
 * Compact status / category badge. Optional leading status dot.
 */
function Badge({
  children,
  tone = 'neutral',
  size = 'md',
  dot = false,
  style,
  ...rest
}) {
  const t = TONES[tone] || TONES.neutral;
  const sz = SIZES[size] || SIZES.md;
  return /*#__PURE__*/React.createElement("span", _extends({
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '6px',
      padding: sz.pad,
      borderRadius: 'var(--radius-full)',
      background: t.bg,
      color: t.fg,
      font: 'var(--font-sans)',
      fontSize: sz.font,
      fontWeight: 'var(--weight-semibold)',
      letterSpacing: 'var(--tracking-tight)',
      lineHeight: 1.4,
      whiteSpace: 'nowrap',
      ...style
    }
  }, rest), dot && /*#__PURE__*/React.createElement("span", {
    style: {
      width: sz.dot,
      height: sz.dot,
      borderRadius: '50%',
      background: t.dot,
      flex: 'none'
    }
  }), children);
}
Object.assign(__ds_scope, { Badge });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/display/Badge.jsx", error: String((e && e.message) || e) }); }

// components/display/Card.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * Card — the primary surface container. Optional header (title/subtitle/
 * actions) and footer. `interactive` adds hover elevation for clickable cards.
 */
function Card({
  title,
  subtitle,
  actions,
  footer,
  children,
  interactive = false,
  padding = 'md',
  onClick,
  style,
  bodyStyle,
  ...rest
}) {
  const [hover, setHover] = React.useState(false);
  const pad = {
    none: 0,
    sm: 'var(--space-8)',
    md: 'var(--space-12)',
    lg: 'var(--space-16)'
  }[padding] ?? 'var(--space-12)';
  return /*#__PURE__*/React.createElement("div", _extends({
    onClick: onClick,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => setHover(false),
    style: {
      background: 'var(--surface-card)',
      border: '1px solid var(--border-default)',
      borderRadius: 'var(--radius-lg)',
      boxShadow: interactive && hover ? 'var(--shadow-md)' : 'var(--shadow-sm)',
      transform: interactive && hover ? 'translateY(-1px)' : 'none',
      transition: 'box-shadow var(--duration-fast) var(--ease-standard), transform var(--duration-fast) var(--ease-standard), border-color var(--duration-fast) var(--ease-standard)',
      borderColor: interactive && hover ? 'var(--border-strong)' : 'var(--border-default)',
      cursor: interactive ? 'pointer' : 'default',
      overflow: 'hidden',
      display: 'flex',
      flexDirection: 'column',
      ...style
    }
  }, rest), (title || actions) && /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'flex-start',
      justifyContent: 'space-between',
      gap: '12px',
      padding: `var(--space-12) ${typeof pad === 'string' ? pad : pad + 'px'}`,
      borderBottom: children ? '1px solid var(--border-subtle)' : 'none'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      minWidth: 0
    }
  }, title && /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--font-title)',
      color: 'var(--text-primary)'
    }
  }, title), subtitle && /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--font-caption)',
      color: 'var(--text-tertiary)',
      marginTop: '2px'
    }
  }, subtitle)), actions && /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 'none',
      display: 'flex',
      gap: '6px'
    }
  }, actions)), children != null && /*#__PURE__*/React.createElement("div", {
    style: {
      padding: pad,
      flex: 1,
      ...bodyStyle
    }
  }, children), footer && /*#__PURE__*/React.createElement("div", {
    style: {
      padding: `var(--space-10) ${typeof pad === 'string' ? pad : pad + 'px'}`,
      borderTop: '1px solid var(--border-subtle)',
      background: 'var(--bg-subtle)'
    }
  }, footer));
}
Object.assign(__ds_scope, { Card });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/display/Card.jsx", error: String((e && e.message) || e) }); }

// components/display/Tag.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * Tag — small removable chip for filters, attributes, and selections.
 * Quieter than Badge; squared corners; optional dismiss button.
 */
function Tag({
  children,
  onRemove,
  color,
  icon,
  style,
  ...rest
}) {
  const [hover, setHover] = React.useState(false);
  return /*#__PURE__*/React.createElement("span", _extends({
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '6px',
      padding: '3px 6px 3px 8px',
      borderRadius: 'var(--radius-sm)',
      background: 'var(--surface-active)',
      color: 'var(--text-secondary)',
      border: '1px solid var(--border-default)',
      font: 'var(--font-sans)',
      fontSize: 'var(--text-xs)',
      fontWeight: 'var(--weight-medium)',
      lineHeight: 1.4,
      whiteSpace: 'nowrap',
      ...style
    }
  }, rest), color && /*#__PURE__*/React.createElement("span", {
    style: {
      width: 7,
      height: 7,
      borderRadius: '2px',
      background: color,
      flex: 'none'
    }
  }), icon, /*#__PURE__*/React.createElement("span", {
    style: {
      paddingRight: onRemove ? 0 : '2px'
    }
  }, children), onRemove && /*#__PURE__*/React.createElement("button", {
    type: "button",
    "aria-label": "Remove",
    onClick: onRemove,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => setHover(false),
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      width: 15,
      height: 15,
      borderRadius: 'var(--radius-xs)',
      border: 'none',
      padding: 0,
      background: hover ? 'var(--surface-overlay)' : 'transparent',
      color: hover ? 'var(--text-primary)' : 'var(--text-tertiary)',
      cursor: 'pointer'
    }
  }, /*#__PURE__*/React.createElement("svg", {
    width: "9",
    height: "9",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "3",
    strokeLinecap: "round"
  }, /*#__PURE__*/React.createElement("path", {
    d: "M18 6 6 18M6 6l12 12"
  }))));
}
Object.assign(__ds_scope, { Tag });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/display/Tag.jsx", error: String((e && e.message) || e) }); }

// components/feedback/Dialog.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * Dialog — centered modal with scrim, header, body and footer actions.
 * Closes on scrim click and Escape.
 */
function Dialog({
  open,
  onClose,
  title,
  description,
  children,
  footer,
  width = 480,
  closeOnScrim = true,
  ...rest
}) {
  React.useEffect(() => {
    if (!open) return;
    const onKey = e => {
      if (e.key === 'Escape') onClose && onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, onClose]);
  if (!open) return null;
  return /*#__PURE__*/React.createElement("div", {
    onMouseDown: e => {
      if (closeOnScrim && e.target === e.currentTarget) onClose && onClose();
    },
    style: {
      position: 'fixed',
      inset: 0,
      zIndex: 'var(--z-modal)',
      background: 'var(--surface-overlay)',
      backdropFilter: 'blur(2px)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      padding: '24px',
      animation: 'frappe-fade var(--duration-fast) var(--ease-standard)'
    }
  }, /*#__PURE__*/React.createElement("style", null, '@keyframes frappe-fade{from{opacity:0}to{opacity:1}}@keyframes frappe-pop{from{opacity:0;transform:translateY(8px) scale(0.98)}to{opacity:1;transform:none}}'), /*#__PURE__*/React.createElement("div", _extends({
    role: "dialog",
    "aria-modal": "true",
    "aria-label": typeof title === 'string' ? title : undefined,
    style: {
      width,
      maxWidth: '100%',
      maxHeight: '90vh',
      display: 'flex',
      flexDirection: 'column',
      background: 'var(--surface-raised)',
      border: '1px solid var(--border-default)',
      borderRadius: 'var(--radius-xl)',
      boxShadow: 'var(--shadow-lg)',
      overflow: 'hidden',
      animation: 'frappe-pop var(--duration-base) var(--ease-out)'
    }
  }, rest), (title || onClose) && /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'flex-start',
      justifyContent: 'space-between',
      gap: '16px',
      padding: '18px 20px 14px'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      minWidth: 0
    }
  }, title && /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--font-h3)',
      color: 'var(--text-primary)'
    }
  }, title), description && /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--font-body)',
      color: 'var(--text-secondary)',
      marginTop: '4px'
    }
  }, description)), onClose && /*#__PURE__*/React.createElement("button", {
    type: "button",
    "aria-label": "Close",
    onClick: onClose,
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      width: 30,
      height: 30,
      borderRadius: 'var(--radius-md)',
      border: 'none',
      flex: 'none',
      background: 'transparent',
      color: 'var(--text-tertiary)',
      cursor: 'pointer'
    }
  }, /*#__PURE__*/React.createElement("svg", {
    width: "16",
    height: "16",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "2.2",
    strokeLinecap: "round"
  }, /*#__PURE__*/React.createElement("path", {
    d: "M18 6 6 18M6 6l12 12"
  })))), children != null && /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '0 20px 16px',
      overflowY: 'auto',
      font: 'var(--font-body)',
      color: 'var(--text-secondary)'
    }
  }, children), footer && /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      justifyContent: 'flex-end',
      gap: '8px',
      padding: '14px 20px',
      borderTop: '1px solid var(--border-subtle)',
      background: 'var(--bg-subtle)'
    }
  }, footer)));
}
Object.assign(__ds_scope, { Dialog });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/feedback/Dialog.jsx", error: String((e && e.message) || e) }); }

// components/feedback/Toast.jsx
try { (() => {
const TONES = {
  neutral: {
    accent: 'var(--text-primary)',
    icon: null
  },
  success: {
    accent: 'var(--success)',
    icon: 'M20 6 9 17l-5-5'
  },
  warning: {
    accent: 'var(--warning)',
    icon: 'M12 9v4M12 17h.01M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z'
  },
  danger: {
    accent: 'var(--danger)',
    icon: 'M12 9v4M12 17h.01M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z'
  },
  info: {
    accent: 'var(--info)',
    icon: 'M12 16v-4M12 8h.01'
  }
};

/**
 * Toast — a single transient notification card. Compose several inside
 * `ToastViewport`. Self-dismisses after `duration` ms (0 = sticky).
 */
function Toast({
  title,
  message,
  tone = 'neutral',
  onClose,
  action,
  duration = 4000
}) {
  const t = TONES[tone] || TONES.neutral;
  React.useEffect(() => {
    if (!duration) return;
    const id = setTimeout(() => onClose && onClose(), duration);
    return () => clearTimeout(id);
  }, [duration, onClose]);
  return /*#__PURE__*/React.createElement("div", {
    role: "status",
    style: {
      display: 'flex',
      alignItems: 'flex-start',
      gap: '11px',
      width: 340,
      maxWidth: '90vw',
      padding: '12px 14px',
      background: 'var(--surface-raised)',
      border: '1px solid var(--border-default)',
      borderLeft: `3px solid ${t.accent}`,
      borderRadius: 'var(--radius-md)',
      boxShadow: 'var(--shadow-lg)',
      animation: 'frappe-toast var(--duration-base) var(--ease-out)'
    }
  }, /*#__PURE__*/React.createElement("style", null, '@keyframes frappe-toast{from{opacity:0;transform:translateX(12px)}to{opacity:1;transform:none}}'), t.icon && /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 'none',
      color: t.accent,
      marginTop: '1px'
    }
  }, /*#__PURE__*/React.createElement("svg", {
    width: "17",
    height: "17",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "2.1",
    strokeLinecap: "round",
    strokeLinejoin: "round"
  }, /*#__PURE__*/React.createElement("path", {
    d: t.icon
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      minWidth: 0
    }
  }, title && /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--font-body-strong)',
      color: 'var(--text-primary)'
    }
  }, title), message && /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--font-caption)',
      color: 'var(--text-secondary)',
      marginTop: '2px'
    }
  }, message), action && /*#__PURE__*/React.createElement("div", {
    style: {
      marginTop: '8px'
    }
  }, action)), onClose && /*#__PURE__*/React.createElement("button", {
    type: "button",
    "aria-label": "Dismiss",
    onClick: onClose,
    style: {
      flex: 'none',
      border: 'none',
      background: 'transparent',
      cursor: 'pointer',
      color: 'var(--text-tertiary)',
      padding: '2px',
      marginTop: '-2px'
    }
  }, /*#__PURE__*/React.createElement("svg", {
    width: "14",
    height: "14",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "2.2",
    strokeLinecap: "round"
  }, /*#__PURE__*/React.createElement("path", {
    d: "M18 6 6 18M6 6l12 12"
  }))));
}

/**
 * ToastViewport — fixed stacking container. Place once near the app root.
 */
function ToastViewport({
  children,
  placement = 'bottom-right'
}) {
  const pos = {
    'bottom-right': {
      bottom: 20,
      right: 20,
      alignItems: 'flex-end'
    },
    'bottom-left': {
      bottom: 20,
      left: 20,
      alignItems: 'flex-start'
    },
    'top-right': {
      top: 20,
      right: 20,
      alignItems: 'flex-end'
    },
    'top-center': {
      top: 20,
      left: '50%',
      transform: 'translateX(-50%)',
      alignItems: 'center'
    }
  }[placement] || {};
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'fixed',
      zIndex: 'var(--z-toast)',
      display: 'flex',
      flexDirection: 'column',
      gap: '10px',
      ...pos
    }
  }, children);
}
Object.assign(__ds_scope, { Toast, ToastViewport });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/feedback/Toast.jsx", error: String((e && e.message) || e) }); }

// components/feedback/Tooltip.jsx
try { (() => {
/**
 * Frappe Tooltip — hover/focus label for icon buttons and truncated text.
 * Lightweight, no portal; positions relative to a wrapped trigger.
 */
function Tooltip({
  label,
  children,
  side = 'top',
  delay = 250
}) {
  const [open, setOpen] = React.useState(false);
  const timer = React.useRef(null);
  const show = () => {
    timer.current = setTimeout(() => setOpen(true), delay);
  };
  const hide = () => {
    clearTimeout(timer.current);
    setOpen(false);
  };
  const pos = {
    top: {
      bottom: '100%',
      left: '50%',
      transform: 'translateX(-50%)',
      marginBottom: '7px'
    },
    bottom: {
      top: '100%',
      left: '50%',
      transform: 'translateX(-50%)',
      marginTop: '7px'
    },
    left: {
      right: '100%',
      top: '50%',
      transform: 'translateY(-50%)',
      marginRight: '7px'
    },
    right: {
      left: '100%',
      top: '50%',
      transform: 'translateY(-50%)',
      marginLeft: '7px'
    }
  }[side] || {};
  return /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'relative',
      display: 'inline-flex'
    },
    onMouseEnter: show,
    onMouseLeave: hide,
    onFocus: show,
    onBlur: hide
  }, children, open && /*#__PURE__*/React.createElement("span", {
    role: "tooltip",
    style: {
      position: 'absolute',
      zIndex: 'var(--z-tooltip)',
      ...pos,
      background: 'var(--neutral-900)',
      color: '#fff',
      font: 'var(--font-caption)',
      fontWeight: 'var(--weight-medium)',
      padding: '5px 8px',
      borderRadius: 'var(--radius-sm)',
      boxShadow: 'var(--shadow-md)',
      whiteSpace: 'nowrap',
      pointerEvents: 'none',
      letterSpacing: 'var(--tracking-tight)',
      animation: 'frappe-tip-in var(--duration-fast) var(--ease-out)'
    }
  }, /*#__PURE__*/React.createElement("style", null, '@keyframes frappe-tip-in{from{opacity:0;transform:' + (pos.transform || '') + ' scale(0.96)}to{opacity:1}}'), label));
}
Object.assign(__ds_scope, { Tooltip });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/feedback/Tooltip.jsx", error: String((e && e.message) || e) }); }

// components/forms/Button.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const SIZES = {
  sm: {
    height: 'var(--control-sm)',
    padding: '0 10px',
    font: 'var(--text-sm)',
    gap: '6px',
    radius: 'var(--radius-sm)'
  },
  md: {
    height: 'var(--control-md)',
    padding: '0 14px',
    font: 'var(--text-md)',
    gap: '7px',
    radius: 'var(--radius-md)'
  },
  lg: {
    height: 'var(--control-lg)',
    padding: '0 18px',
    font: 'var(--text-base)',
    gap: '8px',
    radius: 'var(--radius-md)'
  }
};
const VARIANTS = {
  primary: {
    background: 'var(--brand)',
    color: 'var(--text-on-brand)',
    border: '1px solid transparent',
    boxShadow: 'var(--shadow-xs)',
    '--hover-bg': 'var(--brand-hover)',
    '--active-bg': 'var(--brand-active)'
  },
  secondary: {
    background: 'var(--surface-card)',
    color: 'var(--text-primary)',
    border: '1px solid var(--border-default)',
    boxShadow: 'var(--shadow-xs)',
    '--hover-bg': 'var(--surface-hover)',
    '--active-bg': 'var(--surface-active)'
  },
  subtle: {
    background: 'var(--brand-subtle)',
    color: 'var(--brand-text)',
    border: '1px solid transparent',
    '--hover-bg': 'var(--brand-subtle-hover)',
    '--active-bg': 'var(--brand-subtle-hover)'
  },
  ghost: {
    background: 'transparent',
    color: 'var(--text-secondary)',
    border: '1px solid transparent',
    '--hover-bg': 'var(--surface-hover)',
    '--active-bg': 'var(--surface-active)'
  },
  danger: {
    background: 'var(--danger)',
    color: '#fff',
    border: '1px solid transparent',
    boxShadow: 'var(--shadow-xs)',
    '--hover-bg': 'var(--danger)',
    '--active-bg': 'var(--danger)'
  }
};
const Spinner = () => /*#__PURE__*/React.createElement("span", {
  style: {
    width: '1em',
    height: '1em',
    borderRadius: '50%',
    border: '2px solid currentColor',
    borderTopColor: 'transparent',
    display: 'inline-block',
    animation: 'frappe-spin 0.6s linear infinite',
    opacity: 0.9
  }
});

/**
 * Frappe primary action button. Five variants, three sizes.
 */
function Button({
  children,
  variant = 'secondary',
  size = 'md',
  type = 'button',
  iconLeft,
  iconRight,
  loading = false,
  disabled = false,
  fullWidth = false,
  onClick,
  style,
  ...rest
}) {
  const sz = SIZES[size] || SIZES.md;
  const vr = VARIANTS[variant] || VARIANTS.secondary;
  const isDisabled = disabled || loading;
  const [hover, setHover] = React.useState(false);
  const [active, setActive] = React.useState(false);
  const bg = active ? vr['--active-bg'] : hover ? vr['--hover-bg'] : vr.background;
  return /*#__PURE__*/React.createElement("button", _extends({
    type: type,
    onClick: isDisabled ? undefined : onClick,
    disabled: isDisabled,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => {
      setHover(false);
      setActive(false);
    },
    onMouseDown: () => setActive(true),
    onMouseUp: () => setActive(false),
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      gap: sz.gap,
      height: sz.height,
      padding: sz.padding,
      fontFamily: 'var(--font-sans)',
      fontSize: sz.font,
      fontWeight: 'var(--weight-medium)',
      lineHeight: 1,
      letterSpacing: 'var(--tracking-tight)',
      borderRadius: sz.radius,
      cursor: isDisabled ? 'not-allowed' : 'pointer',
      width: fullWidth ? '100%' : 'auto',
      transition: 'background-color var(--duration-fast) var(--ease-standard), transform var(--duration-instant) var(--ease-standard), box-shadow var(--duration-fast) var(--ease-standard)',
      transform: active && !isDisabled ? 'scale(0.97)' : 'scale(1)',
      opacity: isDisabled ? 0.5 : 1,
      boxShadow: vr.boxShadow,
      color: vr.color,
      border: vr.border,
      background: bg,
      whiteSpace: 'nowrap',
      userSelect: 'none',
      ...style
    }
  }, rest), /*#__PURE__*/React.createElement("style", null, '@keyframes frappe-spin{to{transform:rotate(360deg)}}'), loading ? /*#__PURE__*/React.createElement(Spinner, null) : iconLeft, children && /*#__PURE__*/React.createElement("span", null, children), !loading && iconRight);
}
Object.assign(__ds_scope, { Button });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Button.jsx", error: String((e && e.message) || e) }); }

// components/forms/Checkbox.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * Checkbox with label. Supports indeterminate via the `indeterminate` prop.
 */
function Checkbox({
  checked,
  defaultChecked,
  indeterminate = false,
  onChange,
  label,
  hint,
  disabled = false,
  id,
  ...rest
}) {
  const boxId = id || React.useId();
  const isControlled = checked !== undefined;
  const [internal, setInternal] = React.useState(!!defaultChecked);
  const on = isControlled ? checked : internal;
  const toggle = e => {
    if (disabled) return;
    if (!isControlled) setInternal(e.target.checked);
    onChange && onChange(e);
  };
  return /*#__PURE__*/React.createElement("label", {
    htmlFor: boxId,
    style: {
      display: 'inline-flex',
      alignItems: hint ? 'flex-start' : 'center',
      gap: '9px',
      cursor: disabled ? 'not-allowed' : 'pointer',
      opacity: disabled ? 0.55 : 1
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'relative',
      display: 'inline-flex',
      flex: 'none',
      marginTop: hint ? '1px' : 0
    }
  }, /*#__PURE__*/React.createElement("input", _extends({
    type: "checkbox",
    id: boxId,
    checked: isControlled ? checked : undefined,
    defaultChecked: isControlled ? undefined : defaultChecked,
    onChange: toggle,
    disabled: disabled,
    style: {
      position: 'absolute',
      opacity: 0,
      width: '18px',
      height: '18px',
      margin: 0,
      cursor: 'inherit'
    }
  }, rest)), /*#__PURE__*/React.createElement("span", {
    style: {
      width: '18px',
      height: '18px',
      borderRadius: 'var(--radius-xs)',
      border: `1.5px solid ${on || indeterminate ? 'var(--brand)' : 'var(--border-strong)'}`,
      background: on || indeterminate ? 'var(--brand)' : 'var(--surface-inset)',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      transition: 'background-color var(--duration-fast) var(--ease-standard), border-color var(--duration-fast) var(--ease-standard)',
      color: '#fff'
    }
  }, indeterminate ? /*#__PURE__*/React.createElement("svg", {
    width: "11",
    height: "11",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "3.5",
    strokeLinecap: "round"
  }, /*#__PURE__*/React.createElement("path", {
    d: "M5 12h14"
  })) : on ? /*#__PURE__*/React.createElement("svg", {
    width: "12",
    height: "12",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "3.2",
    strokeLinecap: "round",
    strokeLinejoin: "round"
  }, /*#__PURE__*/React.createElement("path", {
    d: "M20 6 9 17l-5-5"
  })) : null)), label && /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: '2px'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--font-body)',
      color: 'var(--text-primary)'
    }
  }, label), hint && /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--font-caption)',
      color: 'var(--text-tertiary)'
    }
  }, hint)));
}
Object.assign(__ds_scope, { Checkbox });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Checkbox.jsx", error: String((e && e.message) || e) }); }

// components/forms/IconButton.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const SIZES = {
  sm: {
    box: 'var(--control-sm)',
    radius: 'var(--radius-sm)'
  },
  md: {
    box: 'var(--control-md)',
    radius: 'var(--radius-md)'
  },
  lg: {
    box: 'var(--control-lg)',
    radius: 'var(--radius-md)'
  }
};
const VARIANTS = {
  secondary: {
    background: 'var(--surface-card)',
    color: 'var(--text-secondary)',
    border: '1px solid var(--border-default)',
    '--hover-bg': 'var(--surface-hover)'
  },
  ghost: {
    background: 'transparent',
    color: 'var(--text-secondary)',
    border: '1px solid transparent',
    '--hover-bg': 'var(--surface-hover)'
  },
  subtle: {
    background: 'var(--brand-subtle)',
    color: 'var(--brand-text)',
    border: '1px solid transparent',
    '--hover-bg': 'var(--brand-subtle-hover)'
  },
  primary: {
    background: 'var(--brand)',
    color: 'var(--text-on-brand)',
    border: '1px solid transparent',
    '--hover-bg': 'var(--brand-hover)'
  }
};

/**
 * Square icon-only button. Pairs a single glyph with an accessible label.
 */
function IconButton({
  icon,
  label,
  variant = 'ghost',
  size = 'md',
  disabled = false,
  onClick,
  style,
  ...rest
}) {
  const sz = SIZES[size] || SIZES.md;
  const vr = VARIANTS[variant] || VARIANTS.ghost;
  const [hover, setHover] = React.useState(false);
  const [active, setActive] = React.useState(false);
  return /*#__PURE__*/React.createElement("button", _extends({
    type: "button",
    "aria-label": label,
    title: label,
    onClick: disabled ? undefined : onClick,
    disabled: disabled,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => {
      setHover(false);
      setActive(false);
    },
    onMouseDown: () => setActive(true),
    onMouseUp: () => setActive(false),
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      width: sz.box,
      height: sz.box,
      padding: 0,
      borderRadius: sz.radius,
      border: vr.border,
      background: hover && !disabled ? vr['--hover-bg'] : vr.background,
      color: vr.color,
      cursor: disabled ? 'not-allowed' : 'pointer',
      opacity: disabled ? 0.5 : 1,
      transition: 'background-color var(--duration-fast) var(--ease-standard), transform var(--duration-instant) var(--ease-standard)',
      transform: active && !disabled ? 'scale(0.94)' : 'scale(1)',
      ...style
    }
  }, rest), icon);
}
Object.assign(__ds_scope, { IconButton });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/IconButton.jsx", error: String((e && e.message) || e) }); }

// components/forms/Input.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const SIZES = {
  sm: {
    height: 'var(--control-sm)',
    font: 'var(--text-sm)',
    pad: '0 10px'
  },
  md: {
    height: 'var(--control-md)',
    font: 'var(--text-md)',
    pad: '0 12px'
  },
  lg: {
    height: 'var(--control-lg)',
    font: 'var(--text-base)',
    pad: '0 14px'
  }
};

/**
 * Text input with optional label, leading icon, suffix, and error state.
 */
function Input({
  label,
  hint,
  error,
  size = 'md',
  iconLeft,
  suffix,
  mono = false,
  disabled = false,
  id,
  style,
  containerStyle,
  ...rest
}) {
  const sz = SIZES[size] || SIZES.md;
  const [focus, setFocus] = React.useState(false);
  const inputId = id || React.useId();
  const borderColor = error ? 'var(--danger)' : focus ? 'var(--brand)' : 'var(--border-default)';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: '6px',
      ...containerStyle
    }
  }, label && /*#__PURE__*/React.createElement("label", {
    htmlFor: inputId,
    style: {
      font: 'var(--font-label)',
      color: 'var(--text-secondary)'
    }
  }, label), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: '8px',
      height: sz.height,
      padding: sz.pad,
      background: 'var(--surface-inset)',
      border: `1px solid ${borderColor}`,
      borderRadius: 'var(--radius-md)',
      boxShadow: focus ? `0 0 0 3px var(--focus-ring)` : 'none',
      transition: 'border-color var(--duration-fast) var(--ease-standard), box-shadow var(--duration-fast) var(--ease-standard)',
      opacity: disabled ? 0.55 : 1,
      cursor: disabled ? 'not-allowed' : 'text'
    }
  }, iconLeft && /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      color: 'var(--text-tertiary)',
      flex: 'none'
    }
  }, iconLeft), /*#__PURE__*/React.createElement("input", _extends({
    id: inputId,
    disabled: disabled,
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
    style: {
      flex: 1,
      minWidth: 0,
      border: 'none',
      outline: 'none',
      background: 'transparent',
      fontFamily: mono ? 'var(--font-mono)' : 'var(--font-sans)',
      fontSize: sz.font,
      color: 'var(--text-primary)',
      fontVariantNumeric: mono ? 'tabular-nums' : 'normal',
      ...style
    }
  }, rest)), suffix && /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--font-caption)',
      color: 'var(--text-tertiary)',
      flex: 'none'
    }
  }, suffix)), (hint || error) && /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--font-caption)',
      color: error ? 'var(--danger-text)' : 'var(--text-tertiary)'
    }
  }, error || hint));
}
Object.assign(__ds_scope, { Input });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Input.jsx", error: String((e && e.message) || e) }); }

// components/forms/Select.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const SIZES = {
  sm: {
    height: 'var(--control-sm)',
    font: 'var(--text-sm)',
    pad: '0 30px 0 10px'
  },
  md: {
    height: 'var(--control-md)',
    font: 'var(--text-md)',
    pad: '0 32px 0 12px'
  },
  lg: {
    height: 'var(--control-lg)',
    font: 'var(--text-base)',
    pad: '0 34px 0 14px'
  }
};

/**
 * Styled native select with label, hint/error and a chevron affordance.
 */
function Select({
  label,
  hint,
  error,
  size = 'md',
  options = [],
  placeholder,
  disabled = false,
  id,
  value,
  onChange,
  containerStyle,
  style,
  children,
  ...rest
}) {
  const sz = SIZES[size] || SIZES.md;
  const [focus, setFocus] = React.useState(false);
  const selectId = id || React.useId();
  const borderColor = error ? 'var(--danger)' : focus ? 'var(--brand)' : 'var(--border-default)';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: '6px',
      ...containerStyle
    }
  }, label && /*#__PURE__*/React.createElement("label", {
    htmlFor: selectId,
    style: {
      font: 'var(--font-label)',
      color: 'var(--text-secondary)'
    }
  }, label), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      display: 'flex',
      alignItems: 'center'
    }
  }, /*#__PURE__*/React.createElement("select", _extends({
    id: selectId,
    value: value,
    onChange: onChange,
    disabled: disabled,
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
    style: {
      appearance: 'none',
      WebkitAppearance: 'none',
      width: '100%',
      height: sz.height,
      padding: sz.pad,
      fontFamily: 'var(--font-sans)',
      fontSize: sz.font,
      color: 'var(--text-primary)',
      background: 'var(--surface-inset)',
      border: `1px solid ${borderColor}`,
      borderRadius: 'var(--radius-md)',
      cursor: disabled ? 'not-allowed' : 'pointer',
      boxShadow: focus ? '0 0 0 3px var(--focus-ring)' : 'none',
      transition: 'border-color var(--duration-fast) var(--ease-standard), box-shadow var(--duration-fast) var(--ease-standard)',
      opacity: disabled ? 0.55 : 1,
      ...style
    }
  }, rest), placeholder && /*#__PURE__*/React.createElement("option", {
    value: "",
    disabled: true
  }, placeholder), options.map(o => {
    const opt = typeof o === 'string' ? {
      value: o,
      label: o
    } : o;
    return /*#__PURE__*/React.createElement("option", {
      key: opt.value,
      value: opt.value
    }, opt.label);
  }), children), /*#__PURE__*/React.createElement("svg", {
    width: "14",
    height: "14",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "2.2",
    strokeLinecap: "round",
    strokeLinejoin: "round",
    style: {
      position: 'absolute',
      right: '11px',
      color: 'var(--text-tertiary)',
      pointerEvents: 'none'
    }
  }, /*#__PURE__*/React.createElement("path", {
    d: "m6 9 6 6 6-6"
  }))), (hint || error) && /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--font-caption)',
      color: error ? 'var(--danger-text)' : 'var(--text-tertiary)'
    }
  }, error || hint));
}
Object.assign(__ds_scope, { Select });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Select.jsx", error: String((e && e.message) || e) }); }

// components/forms/Switch.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const SIZES = {
  sm: {
    w: 32,
    h: 18,
    knob: 12
  },
  md: {
    w: 40,
    h: 22,
    knob: 16
  }
};

/**
 * On/off switch. Use for instant-apply settings (not form submission).
 */
function Switch({
  checked,
  defaultChecked,
  onChange,
  label,
  size = 'md',
  disabled = false,
  id,
  ...rest
}) {
  const sz = SIZES[size] || SIZES.md;
  const switchId = id || React.useId();
  const isControlled = checked !== undefined;
  const [internal, setInternal] = React.useState(!!defaultChecked);
  const on = isControlled ? checked : internal;
  const toggle = e => {
    if (disabled) return;
    if (!isControlled) setInternal(e.target.checked);
    onChange && onChange(e);
  };
  const pad = (sz.h - sz.knob) / 2;
  return /*#__PURE__*/React.createElement("label", {
    htmlFor: switchId,
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '10px',
      cursor: disabled ? 'not-allowed' : 'pointer',
      opacity: disabled ? 0.55 : 1
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'relative',
      display: 'inline-flex',
      flex: 'none'
    }
  }, /*#__PURE__*/React.createElement("input", _extends({
    type: "checkbox",
    id: switchId,
    checked: isControlled ? checked : undefined,
    defaultChecked: isControlled ? undefined : defaultChecked,
    onChange: toggle,
    disabled: disabled,
    style: {
      position: 'absolute',
      opacity: 0,
      width: sz.w,
      height: sz.h,
      margin: 0,
      cursor: 'inherit'
    }
  }, rest)), /*#__PURE__*/React.createElement("span", {
    style: {
      width: sz.w,
      height: sz.h,
      borderRadius: 'var(--radius-full)',
      background: on ? 'var(--brand)' : 'var(--border-strong)',
      transition: 'background-color var(--duration-base) var(--ease-standard)',
      display: 'inline-block',
      position: 'relative'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      top: pad,
      left: pad,
      width: sz.knob,
      height: sz.knob,
      borderRadius: '50%',
      background: '#fff',
      boxShadow: 'var(--shadow-sm)',
      transform: on ? `translateX(${sz.w - sz.knob - pad * 2}px)` : 'translateX(0)',
      transition: 'transform var(--duration-base) var(--ease-out)'
    }
  }))), label && /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--font-body)',
      color: 'var(--text-primary)'
    }
  }, label));
}
Object.assign(__ds_scope, { Switch });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Switch.jsx", error: String((e && e.message) || e) }); }

// components/navigation/Tabs.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * Tabs — controlled or uncontrolled tab strip. Two looks: `underline`
 * (page-level navigation) and `pill` (segmented, for filters/sub-views).
 */
function Tabs({
  tabs = [],
  value,
  defaultValue,
  onChange,
  variant = 'underline',
  size = 'md',
  style,
  ...rest
}) {
  const isControlled = value !== undefined;
  const first = defaultValue ?? (tabs[0] && (tabs[0].value ?? tabs[0]));
  const [internal, setInternal] = React.useState(first);
  const active = isControlled ? value : internal;
  const select = v => {
    if (!isControlled) setInternal(v);
    onChange && onChange(v);
  };
  const fontSize = size === 'sm' ? 'var(--text-sm)' : 'var(--text-md)';
  const padY = size === 'sm' ? '6px' : '9px';
  if (variant === 'pill') {
    return /*#__PURE__*/React.createElement("div", _extends({
      role: "tablist",
      style: {
        display: 'inline-flex',
        gap: '2px',
        padding: '3px',
        background: 'var(--bg-subtle)',
        border: '1px solid var(--border-default)',
        borderRadius: 'var(--radius-md)',
        ...style
      }
    }, rest), tabs.map(t => {
      const tab = typeof t === 'string' ? {
        value: t,
        label: t
      } : t;
      const on = tab.value === active;
      return /*#__PURE__*/React.createElement("button", {
        key: tab.value,
        role: "tab",
        "aria-selected": on,
        onClick: () => select(tab.value),
        style: {
          display: 'inline-flex',
          alignItems: 'center',
          gap: '6px',
          padding: `${size === 'sm' ? '4px 10px' : '6px 12px'}`,
          border: 'none',
          borderRadius: 'var(--radius-sm)',
          cursor: 'pointer',
          font: 'var(--font-sans)',
          fontSize,
          fontWeight: 'var(--weight-medium)',
          background: on ? 'var(--surface-card)' : 'transparent',
          color: on ? 'var(--text-primary)' : 'var(--text-secondary)',
          boxShadow: on ? 'var(--shadow-xs)' : 'none',
          transition: 'background-color var(--duration-fast) var(--ease-standard), color var(--duration-fast) var(--ease-standard)'
        }
      }, tab.icon, tab.label, tab.count != null && /*#__PURE__*/React.createElement(CountPill, {
        on: on
      }, tab.count));
    }));
  }
  return /*#__PURE__*/React.createElement("div", _extends({
    role: "tablist",
    style: {
      display: 'flex',
      gap: '4px',
      borderBottom: '1px solid var(--border-default)',
      ...style
    }
  }, rest), tabs.map(t => {
    const tab = typeof t === 'string' ? {
      value: t,
      label: t
    } : t;
    const on = tab.value === active;
    return /*#__PURE__*/React.createElement("button", {
      key: tab.value,
      role: "tab",
      "aria-selected": on,
      onClick: () => select(tab.value),
      style: {
        display: 'inline-flex',
        alignItems: 'center',
        gap: '7px',
        padding: `${padY} 10px`,
        border: 'none',
        background: 'transparent',
        cursor: 'pointer',
        font: 'var(--font-sans)',
        fontSize,
        fontWeight: 'var(--weight-medium)',
        color: on ? 'var(--text-primary)' : 'var(--text-secondary)',
        borderBottom: `2px solid ${on ? 'var(--brand)' : 'transparent'}`,
        marginBottom: '-1px',
        transition: 'color var(--duration-fast) var(--ease-standard), border-color var(--duration-fast) var(--ease-standard)'
      }
    }, tab.icon, tab.label, tab.count != null && /*#__PURE__*/React.createElement(CountPill, {
      on: on
    }, tab.count));
  }));
}
function CountPill({
  children,
  on
}) {
  return /*#__PURE__*/React.createElement("span", {
    style: {
      minWidth: 18,
      height: 18,
      padding: '0 5px',
      borderRadius: 'var(--radius-full)',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontSize: 'var(--text-2xs)',
      fontWeight: 'var(--weight-semibold)',
      fontVariantNumeric: 'tabular-nums',
      background: on ? 'var(--brand-subtle)' : 'var(--surface-active)',
      color: on ? 'var(--brand-text)' : 'var(--text-tertiary)'
    }
  }, children);
}
Object.assign(__ds_scope, { Tabs });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/navigation/Tabs.jsx", error: String((e && e.message) || e) }); }

// ui_kits/print-shop/AppShell.jsx
try { (() => {
/* App shell — sidebar, topbar, theme toggle */
(function () {
  const {
    Avatar,
    Badge,
    IconButton
  } = window.FrappeDesignSystem_75694f;
  const Tooltip = window.FrappeDesignSystem_75694f.Tooltip || (({
    children
  }) => children);
  const {
    Ic
  } = window.FK;
  const NAV = [{
    key: 'dashboard',
    label: 'Dashboard',
    icon: 'LayoutDashboard'
  }, {
    key: 'orders',
    label: 'Orders',
    icon: 'ClipboardList',
    count: 18
  }, {
    key: 'production',
    label: 'Production',
    icon: 'Factory',
    count: 7
  }, {
    key: 'invoicing',
    label: 'Invoicing',
    icon: 'Receipt',
    count: 3
  }, {
    key: 'inventory',
    label: 'Inventory',
    icon: 'Boxes'
  }, {
    key: 'customers',
    label: 'Customers',
    icon: 'Users'
  }];
  function NavItem({
    item,
    active,
    onClick
  }) {
    const [hover, setHover] = React.useState(false);
    return /*#__PURE__*/React.createElement("button", {
      key: active ? 'on' : 'off',
      onClick: onClick,
      onMouseEnter: () => setHover(true),
      onMouseLeave: () => setHover(false),
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: '10px',
        width: '100%',
        padding: '8px 10px',
        border: 'none',
        cursor: 'pointer',
        textAlign: 'left',
        borderRadius: 'var(--radius-md)',
        fontFamily: 'var(--font-sans)',
        fontWeight: 500,
        fontSize: 'var(--text-md)',
        lineHeight: 1.5,
        backgroundColor: active ? 'var(--brand-subtle)' : hover ? 'var(--surface-hover)' : 'transparent',
        color: active ? 'var(--brand-text)' : 'var(--text-secondary)',
        transition: 'background-color var(--duration-fast), color var(--duration-fast)'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'inline-flex',
        color: active ? 'var(--brand)' : 'var(--text-tertiary)'
      }
    }, /*#__PURE__*/React.createElement(Ic, {
      n: item.icon,
      size: 17
    })), /*#__PURE__*/React.createElement("span", {
      style: {
        flex: 1
      }
    }, item.label), item.count != null && /*#__PURE__*/React.createElement(Badge, {
      tone: active ? 'brand' : 'neutral',
      size: "sm"
    }, item.count));
  }
  function Sidebar({
    view,
    setView
  }) {
    return /*#__PURE__*/React.createElement("aside", {
      style: {
        width: 'var(--sidebar-width)',
        flexShrink: 0,
        background: 'var(--surface-card)',
        borderRight: '1px solid var(--border-default)',
        display: 'flex',
        flexDirection: 'column',
        height: '100%'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: '9px',
        padding: '14px 16px',
        height: 'var(--topbar-height)',
        boxSizing: 'border-box',
        borderBottom: '1px solid var(--border-subtle)'
      }
    }, /*#__PURE__*/React.createElement("img", {
      src: "../../assets/frappe-logo.svg",
      width: "24",
      height: "23",
      alt: "Frappe",
      style: {
        display: 'block'
      }
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--font-title)',
        fontWeight: 700,
        color: 'var(--text-primary)',
        letterSpacing: 'var(--tracking-tight)'
      }
    }, "Frappe"), /*#__PURE__*/React.createElement("span", {
      style: {
        marginLeft: 'auto',
        font: 'var(--font-caption)',
        color: 'var(--text-tertiary)'
      }
    }, "\u2318K")), /*#__PURE__*/React.createElement("div", {
      style: {
        padding: '10px',
        display: 'flex',
        flexDirection: 'column',
        gap: '2px',
        flex: 1,
        overflowY: 'auto'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--font-caption)',
        fontWeight: 600,
        textTransform: 'uppercase',
        letterSpacing: 'var(--tracking-caps)',
        color: 'var(--text-tertiary)',
        padding: '8px 10px 4px'
      }
    }, "Shop"), NAV.map(item => /*#__PURE__*/React.createElement(NavItem, {
      key: item.key,
      item: item,
      active: view === item.key,
      onClick: () => setView(item.key)
    }))), /*#__PURE__*/React.createElement("div", {
      style: {
        padding: '10px',
        borderTop: '1px solid var(--border-subtle)'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: '9px',
        padding: '6px 8px'
      }
    }, /*#__PURE__*/React.createElement(Avatar, {
      name: "Max Bowen",
      size: "sm"
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1,
        minWidth: 0
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--font-label)',
        color: 'var(--text-primary)',
        whiteSpace: 'nowrap',
        overflow: 'hidden',
        textOverflow: 'ellipsis'
      }
    }, "Max Bowen"), /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--font-caption)',
        color: 'var(--text-tertiary)'
      }
    }, "Bowen Print Co.")), /*#__PURE__*/React.createElement("span", {
      style: {
        color: 'var(--text-tertiary)'
      }
    }, /*#__PURE__*/React.createElement(Ic, {
      n: "ChevronsUpDown",
      size: 15
    })))));
  }
  function Topbar({
    theme,
    toggleTheme
  }) {
    return /*#__PURE__*/React.createElement("header", {
      style: {
        height: 'var(--topbar-height)',
        flexShrink: 0,
        borderBottom: '1px solid var(--border-default)',
        background: 'var(--surface-card)',
        display: 'flex',
        alignItems: 'center',
        gap: '12px',
        padding: '0 18px'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        position: 'relative',
        width: '320px',
        maxWidth: '40vw'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        position: 'absolute',
        left: '10px',
        top: '50%',
        transform: 'translateY(-50%)',
        color: 'var(--text-tertiary)',
        pointerEvents: 'none',
        display: 'flex'
      }
    }, /*#__PURE__*/React.createElement(Ic, {
      n: "Search",
      size: 15
    })), /*#__PURE__*/React.createElement("input", {
      placeholder: "Search orders, customers, invoices\u2026",
      style: {
        width: '100%',
        height: 'var(--control-md)',
        boxSizing: 'border-box',
        padding: '0 10px 0 32px',
        borderRadius: 'var(--radius-md)',
        border: '1px solid var(--border-default)',
        background: 'var(--surface-inset)',
        color: 'var(--text-primary)',
        font: 'var(--font-body)',
        outline: 'none'
      }
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1
      }
    }), /*#__PURE__*/React.createElement(Tooltip, {
      label: "Sync with QuickBooks"
    }, /*#__PURE__*/React.createElement(IconButton, {
      icon: /*#__PURE__*/React.createElement(Ic, {
        n: "RefreshCw",
        size: 16
      }),
      label: "Sync",
      variant: "ghost"
    })), /*#__PURE__*/React.createElement(Tooltip, {
      label: "Notifications"
    }, /*#__PURE__*/React.createElement(IconButton, {
      icon: /*#__PURE__*/React.createElement(Ic, {
        n: "Bell",
        size: 16
      }),
      label: "Notifications",
      variant: "ghost"
    })), /*#__PURE__*/React.createElement(Tooltip, {
      label: theme === 'dark' ? 'Light mode' : 'Dark mode'
    }, /*#__PURE__*/React.createElement(IconButton, {
      icon: /*#__PURE__*/React.createElement(Ic, {
        n: theme === 'dark' ? 'Sun' : 'Moon',
        size: 16
      }),
      label: "Toggle theme",
      variant: "ghost",
      onClick: toggleTheme
    })));
  }
  function AppShell({
    view,
    setView,
    theme,
    toggleTheme,
    children
  }) {
    return /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        height: '100vh',
        width: '100vw',
        overflow: 'hidden',
        background: 'var(--bg-base)'
      }
    }, /*#__PURE__*/React.createElement(Sidebar, {
      view: view,
      setView: setView
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1,
        display: 'flex',
        flexDirection: 'column',
        minWidth: 0
      }
    }, /*#__PURE__*/React.createElement(Topbar, {
      theme: theme,
      toggleTheme: toggleTheme
    }), /*#__PURE__*/React.createElement("main", {
      style: {
        flex: 1,
        overflowY: 'auto',
        padding: '24px 28px'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        maxWidth: '1200px',
        margin: '0 auto'
      }
    }, children))));
  }
  window.AppShell = AppShell;
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/print-shop/AppShell.jsx", error: String((e && e.message) || e) }); }

// ui_kits/print-shop/Dashboard.jsx
try { (() => {
/* Dashboard screen */
(function () {
  const {
    Card,
    Badge,
    Avatar,
    AvatarGroup,
    Button
  } = window.FrappeDesignSystem_75694f;
  const {
    Ic,
    Eyebrow,
    Kpi,
    PageHeader
  } = window.FK;
  const D = window.FrappeData;
  function Dashboard({
    onOpenOrder
  }) {
    const recent = D.orders.slice(0, 6);
    return /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement(PageHeader, {
      title: "Good morning, Max",
      subtitle: "Wednesday, June 19 \xB7 7 jobs on the floor, 2 due today",
      actions: /*#__PURE__*/React.createElement(Button, {
        variant: "primary",
        iconLeft: /*#__PURE__*/React.createElement(Ic, {
          n: "Plus",
          size: 15
        })
      }, "New order")
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        gap: '14px',
        marginBottom: '22px'
      }
    }, /*#__PURE__*/React.createElement(Kpi, {
      label: "Open orders",
      value: "18",
      delta: "+3 this week",
      icon: "ClipboardList"
    }), /*#__PURE__*/React.createElement(Kpi, {
      label: "Due today",
      value: "2",
      delta: "1 rush",
      deltaTone: "danger",
      icon: "CalendarClock"
    }), /*#__PURE__*/React.createElement(Kpi, {
      label: "Awaiting art",
      value: "4",
      delta: "Oldest 2d",
      deltaTone: "neutral",
      icon: "Image"
    }), /*#__PURE__*/React.createElement(Kpi, {
      label: "Revenue (MTD)",
      value: "$24.6k",
      delta: "+12% vs May",
      icon: "TrendingUp"
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'grid',
        gridTemplateColumns: '1.7fr 1fr',
        gap: '16px',
        alignItems: 'start'
      }
    }, /*#__PURE__*/React.createElement(Card, {
      padding: "none"
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '14px 16px',
        borderBottom: '1px solid var(--border-subtle)'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--font-title)',
        color: 'var(--text-primary)'
      }
    }, "Recent orders"), /*#__PURE__*/React.createElement(Button, {
      variant: "ghost",
      size: "sm",
      iconRight: /*#__PURE__*/React.createElement(Ic, {
        n: "ArrowRight",
        size: 14
      })
    }, "View all")), /*#__PURE__*/React.createElement("table", {
      style: {
        width: '100%',
        borderCollapse: 'collapse'
      }
    }, /*#__PURE__*/React.createElement("thead", null, /*#__PURE__*/React.createElement("tr", null, ['Order', 'Customer', 'Job', 'Status', 'Total'].map((h, i) => /*#__PURE__*/React.createElement("th", {
      key: h,
      style: {
        textAlign: i === 4 ? 'right' : 'left',
        font: 'var(--font-caption)',
        fontWeight: 600,
        textTransform: 'uppercase',
        letterSpacing: 'var(--tracking-caps)',
        color: 'var(--text-tertiary)',
        padding: '8px 16px',
        borderBottom: '1px solid var(--border-subtle)'
      }
    }, h)))), /*#__PURE__*/React.createElement("tbody", null, recent.map(o => {
      const b = D.statusBadge[o.status];
      return /*#__PURE__*/React.createElement("tr", {
        key: o.id,
        onClick: () => onOpenOrder(o),
        style: {
          cursor: 'pointer',
          transition: 'background var(--duration-fast)'
        },
        onMouseEnter: e => e.currentTarget.style.background = 'var(--surface-hover)',
        onMouseLeave: e => e.currentTarget.style.background = 'transparent'
      }, /*#__PURE__*/React.createElement("td", {
        style: {
          padding: '11px 16px',
          borderBottom: '1px solid var(--border-subtle)',
          fontFamily: 'var(--font-mono)',
          fontWeight: 500,
          color: 'var(--text-primary)'
        },
        className: "tabular"
      }, "#", o.id), /*#__PURE__*/React.createElement("td", {
        style: {
          padding: '11px 16px',
          borderBottom: '1px solid var(--border-subtle)',
          font: 'var(--font-body-strong)',
          color: 'var(--text-primary)'
        }
      }, o.customer), /*#__PURE__*/React.createElement("td", {
        style: {
          padding: '11px 16px',
          borderBottom: '1px solid var(--border-subtle)',
          font: 'var(--font-body)',
          color: 'var(--text-secondary)',
          maxWidth: '180px',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap'
        }
      }, o.job), /*#__PURE__*/React.createElement("td", {
        style: {
          padding: '11px 16px',
          borderBottom: '1px solid var(--border-subtle)'
        }
      }, /*#__PURE__*/React.createElement(Badge, {
        tone: b.tone,
        dot: true,
        size: "sm"
      }, b.label)), /*#__PURE__*/React.createElement("td", {
        style: {
          padding: '11px 16px',
          borderBottom: '1px solid var(--border-subtle)',
          textAlign: 'right',
          fontFamily: 'var(--font-mono)',
          fontWeight: 500,
          color: 'var(--text-primary)'
        },
        className: "tabular"
      }, D.money(o.total)));
    })))), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        gap: '16px'
      }
    }, /*#__PURE__*/React.createElement(Card, {
      title: "On the floor"
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        gap: '12px'
      }
    }, D.columns.slice(0, 4).map(c => {
      const count = D.orders.filter(o => o.stage === c.key).length;
      return /*#__PURE__*/React.createElement("div", {
        key: c.key,
        style: {
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between'
        }
      }, /*#__PURE__*/React.createElement("div", {
        style: {
          display: 'flex',
          alignItems: 'center',
          gap: '8px'
        }
      }, /*#__PURE__*/React.createElement(Badge, {
        tone: c.tone,
        dot: true,
        size: "sm"
      }, c.key)), /*#__PURE__*/React.createElement("span", {
        style: {
          fontFamily: 'var(--font-mono)',
          fontWeight: 600,
          color: 'var(--text-primary)'
        },
        className: "tabular"
      }, count));
    }))), /*#__PURE__*/React.createElement(Card, {
      title: "Today's crew"
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between'
      }
    }, /*#__PURE__*/React.createElement(AvatarGroup, {
      names: ['Max Bowen', 'Dana Ruiz', 'Priya Shah', 'Lee Ortiz', 'Sam Kade'],
      max: 4
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--font-caption)',
        color: 'var(--text-tertiary)'
      }
    }, "5 operators"))))));
  }
  window.Dashboard = Dashboard;
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/print-shop/Dashboard.jsx", error: String((e && e.message) || e) }); }

// ui_kits/print-shop/Invoicing.jsx
try { (() => {
/* Invoicing screen */
(function () {
  const {
    Card,
    Badge,
    Button,
    Tabs,
    Input
  } = window.FrappeDesignSystem_75694f;
  const {
    Ic,
    Kpi,
    PageHeader
  } = window.FK;
  const D = window.FrappeData;
  function Invoicing() {
    const [filter, setFilter] = React.useState('All');
    const list = D.invoices.filter(i => filter === 'All' ? true : filter === 'Unpaid' ? i.status === 'sent' || i.status === 'overdue' || i.status === 'deposit' : filter === 'Paid' ? i.status === 'paid' : filter === 'Overdue' ? i.status === 'overdue' : filter === 'Draft' ? i.status === 'draft' : true);
    const th = (label, align) => /*#__PURE__*/React.createElement("th", {
      style: {
        textAlign: align || 'left',
        font: 'var(--font-caption)',
        fontWeight: 600,
        textTransform: 'uppercase',
        letterSpacing: 'var(--tracking-caps)',
        color: 'var(--text-tertiary)',
        padding: '9px 16px',
        borderBottom: '1px solid var(--border-subtle)'
      }
    }, label);
    return /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement(PageHeader, {
      title: "Invoicing",
      subtitle: "Deposits, balances and payments",
      actions: /*#__PURE__*/React.createElement(Button, {
        variant: "primary",
        iconLeft: /*#__PURE__*/React.createElement(Ic, {
          n: "Plus",
          size: 15
        })
      }, "New invoice")
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        gap: '14px',
        marginBottom: '22px'
      }
    }, /*#__PURE__*/React.createElement(Kpi, {
      label: "Outstanding",
      value: "$583",
      delta: "3 invoices",
      deltaTone: "neutral",
      icon: "Receipt"
    }), /*#__PURE__*/React.createElement(Kpi, {
      label: "Overdue",
      value: "$159",
      delta: "1 invoice \xB7 9d",
      deltaTone: "danger",
      icon: "AlertCircle"
    }), /*#__PURE__*/React.createElement(Kpi, {
      label: "Deposits held",
      value: "$270",
      delta: "Pine & Oak",
      deltaTone: "neutral",
      icon: "PiggyBank"
    }), /*#__PURE__*/React.createElement(Kpi, {
      label: "Paid (MTD)",
      value: "$1.8k",
      delta: "+12% vs May",
      icon: "CheckCircle2"
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        marginBottom: '14px'
      }
    }, /*#__PURE__*/React.createElement(Tabs, {
      variant: "pill",
      value: filter,
      onChange: setFilter,
      tabs: ['All', 'Unpaid', 'Paid', 'Overdue', 'Draft']
    })), /*#__PURE__*/React.createElement(Card, {
      padding: "none"
    }, /*#__PURE__*/React.createElement("table", {
      style: {
        width: '100%',
        borderCollapse: 'collapse'
      }
    }, /*#__PURE__*/React.createElement("thead", null, /*#__PURE__*/React.createElement("tr", null, th('Invoice'), th('Customer'), th('Order'), th('Date'), th('Deposit', 'right'), th('Amount', 'right'), th('Status'), th('', 'right'))), /*#__PURE__*/React.createElement("tbody", null, list.map(inv => {
      const b = D.invoiceBadge[inv.status];
      const td = (children, extra) => /*#__PURE__*/React.createElement("td", {
        style: {
          padding: '12px 16px',
          borderBottom: '1px solid var(--border-subtle)',
          font: 'var(--font-body)',
          color: 'var(--text-secondary)',
          ...extra
        }
      }, children);
      return /*#__PURE__*/React.createElement("tr", {
        key: inv.id,
        onMouseEnter: e => e.currentTarget.style.background = 'var(--surface-hover)',
        onMouseLeave: e => e.currentTarget.style.background = 'transparent'
      }, td(/*#__PURE__*/React.createElement("span", {
        style: {
          fontFamily: 'var(--font-mono)',
          fontWeight: 500,
          color: 'var(--text-primary)'
        },
        className: "tabular"
      }, inv.id)), td(/*#__PURE__*/React.createElement("span", {
        style: {
          font: 'var(--font-body-strong)',
          color: 'var(--text-primary)'
        }
      }, inv.customer)), td(/*#__PURE__*/React.createElement("span", {
        style: {
          fontFamily: 'var(--font-mono)'
        },
        className: "tabular"
      }, "#", inv.order)), td(inv.date), td(/*#__PURE__*/React.createElement("span", {
        style: {
          fontFamily: 'var(--font-mono)',
          color: inv.deposit ? 'var(--text-primary)' : 'var(--text-disabled)'
        },
        className: "tabular"
      }, inv.deposit ? D.money(inv.deposit) : '—'), {
        textAlign: 'right'
      }), td(/*#__PURE__*/React.createElement("span", {
        style: {
          fontFamily: 'var(--font-mono)',
          fontWeight: 500,
          color: 'var(--text-primary)'
        },
        className: "tabular"
      }, D.money(inv.amount)), {
        textAlign: 'right'
      }), td(/*#__PURE__*/React.createElement(Badge, {
        tone: b.tone,
        dot: true,
        size: "sm"
      }, b.label)), td(inv.status === 'draft' ? /*#__PURE__*/React.createElement(Button, {
        variant: "subtle",
        size: "sm"
      }, "Send") : inv.status === 'paid' ? /*#__PURE__*/React.createElement("span", {
        style: {
          color: 'var(--text-disabled)',
          font: 'var(--font-caption)'
        }
      }, "Paid") : /*#__PURE__*/React.createElement(Button, {
        variant: "secondary",
        size: "sm"
      }, "Record payment"), {
        textAlign: 'right'
      }));
    })))));
  }
  window.Invoicing = Invoicing;
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/print-shop/Invoicing.jsx", error: String((e && e.message) || e) }); }

// ui_kits/print-shop/JobTicket.jsx
try { (() => {
/* Job ticket / docket — order detail view */
(function () {
  const {
    Card,
    Badge,
    Button,
    Avatar,
    AvatarGroup,
    Tag,
    IconButton
  } = window.FrappeDesignSystem_75694f;
  const Tooltip = window.FrappeDesignSystem_75694f.Tooltip || (({
    children
  }) => children);
  const {
    Ic,
    Eyebrow
  } = window.FK;
  const D = window.FrappeData;
  function Row({
    label,
    children
  }) {
    return /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        padding: '9px 0',
        borderBottom: '1px solid var(--border-subtle)'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--font-body)',
        color: 'var(--text-tertiary)'
      }
    }, label), /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--font-body-strong)',
        color: 'var(--text-primary)'
      }
    }, children));
  }
  function Step({
    label,
    tone,
    done,
    current
  }) {
    return /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: '10px'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        width: '18px',
        height: '18px',
        borderRadius: '999px',
        flexShrink: 0,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: current ? 'var(--brand)' : done ? 'var(--success)' : 'var(--surface-inset)',
        border: current || done ? 'none' : '1.5px solid var(--border-strong)',
        color: '#fff'
      }
    }, done && !current ? /*#__PURE__*/React.createElement(Ic, {
      n: "Check",
      size: 11
    }) : current ? /*#__PURE__*/React.createElement("span", {
      style: {
        width: '6px',
        height: '6px',
        borderRadius: '999px',
        background: '#fff'
      }
    }) : null), /*#__PURE__*/React.createElement("span", {
      style: {
        font: current ? 'var(--font-body-strong)' : 'var(--font-body)',
        color: current ? 'var(--text-primary)' : done ? 'var(--text-secondary)' : 'var(--text-tertiary)'
      }
    }, label));
  }
  function JobTicket({
    order,
    onBack
  }) {
    const o = order;
    const b = D.statusBadge[o.status];
    const stages = ['Queued', 'Prepress', 'On press', 'Bindery', 'Shipped'];
    const curIdx = stages.indexOf(o.stage);
    return /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: '10px',
        marginBottom: '18px'
      }
    }, /*#__PURE__*/React.createElement(Button, {
      variant: "ghost",
      size: "sm",
      iconLeft: /*#__PURE__*/React.createElement(Ic, {
        n: "ArrowLeft",
        size: 15
      }),
      onClick: onBack
    }, "Orders"), /*#__PURE__*/React.createElement("span", {
      style: {
        color: 'var(--text-disabled)'
      }
    }, "/"), /*#__PURE__*/React.createElement("span", {
      style: {
        fontFamily: 'var(--font-mono)',
        fontWeight: 600,
        color: 'var(--text-secondary)'
      },
      className: "tabular"
    }, "#", o.id)), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'flex-start',
        justifyContent: 'space-between',
        gap: '16px',
        marginBottom: '22px'
      }
    }, /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: '10px',
        marginBottom: '6px'
      }
    }, /*#__PURE__*/React.createElement("h1", {
      style: {
        margin: 0,
        font: 'var(--font-h2)',
        color: 'var(--text-primary)',
        letterSpacing: 'var(--tracking-tight)'
      }
    }, o.job), /*#__PURE__*/React.createElement(Badge, {
      tone: b.tone,
      dot: true
    }, b.label), o.rush && /*#__PURE__*/React.createElement(Badge, {
      tone: "brand"
    }, "Rush")), /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--font-body)',
        color: 'var(--text-secondary)'
      }
    }, o.customer, " \xB7 ", o.contact)), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        gap: '8px',
        flexShrink: 0
      }
    }, /*#__PURE__*/React.createElement(Tooltip, {
      label: "Print job ticket"
    }, /*#__PURE__*/React.createElement(IconButton, {
      icon: /*#__PURE__*/React.createElement(Ic, {
        n: "Printer",
        size: 16
      }),
      label: "Print",
      variant: "secondary"
    })), /*#__PURE__*/React.createElement(Button, {
      variant: "secondary",
      iconLeft: /*#__PURE__*/React.createElement(Ic, {
        n: "Image",
        size: 15
      })
    }, "View proof"), /*#__PURE__*/React.createElement(Button, {
      variant: "primary",
      iconLeft: /*#__PURE__*/React.createElement(Ic, {
        n: "Check",
        size: 15
      })
    }, "Advance stage"))), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'grid',
        gridTemplateColumns: '1.5fr 1fr',
        gap: '16px',
        alignItems: 'start'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        gap: '16px'
      }
    }, /*#__PURE__*/React.createElement(Card, {
      title: "Specifications"
    }, /*#__PURE__*/React.createElement(Row, {
      label: "Quantity"
    }, /*#__PURE__*/React.createElement("span", {
      className: "tabular",
      style: {
        fontFamily: 'var(--font-mono)'
      }
    }, o.qty.toLocaleString(), " units")), /*#__PURE__*/React.createElement(Row, {
      label: "Stock"
    }, o.stock), /*#__PURE__*/React.createElement(Row, {
      label: "Finishing"
    }, "Trim \xB7 Round corners"), /*#__PURE__*/React.createElement(Row, {
      label: "Proof"
    }, "Customer approval required"), /*#__PURE__*/React.createElement("div", {
      style: {
        paddingTop: '9px'
      }
    }, /*#__PURE__*/React.createElement(Row, {
      label: "Due"
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        color: o.status === 'overdue' ? 'var(--danger-text)' : 'var(--text-primary)'
      }
    }, o.due)))), /*#__PURE__*/React.createElement(Card, {
      title: "Pricing"
    }, /*#__PURE__*/React.createElement(Row, {
      label: "Printing"
    }, /*#__PURE__*/React.createElement("span", {
      className: "tabular",
      style: {
        fontFamily: 'var(--font-mono)'
      }
    }, D.money(o.total * 0.78))), /*#__PURE__*/React.createElement(Row, {
      label: "Finishing"
    }, /*#__PURE__*/React.createElement("span", {
      className: "tabular",
      style: {
        fontFamily: 'var(--font-mono)'
      }
    }, D.money(o.total * 0.14))), /*#__PURE__*/React.createElement(Row, {
      label: "Rush surcharge"
    }, /*#__PURE__*/React.createElement("span", {
      className: "tabular",
      style: {
        fontFamily: 'var(--font-mono)'
      }
    }, o.rush ? D.money(o.total * 0.08) : '$0.00')), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        paddingTop: '12px'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--font-title)',
        color: 'var(--text-primary)'
      }
    }, "Total"), /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--font-h3)',
        fontFamily: 'var(--font-mono)',
        color: 'var(--text-primary)'
      },
      className: "tabular"
    }, D.money(o.total))))), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        gap: '16px'
      }
    }, /*#__PURE__*/React.createElement(Card, {
      title: "Production stage"
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        gap: '13px'
      }
    }, stages.map((s, i) => /*#__PURE__*/React.createElement(Step, {
      key: s,
      label: s,
      done: i <= curIdx,
      current: i === curIdx
    })))), /*#__PURE__*/React.createElement(Card, {
      title: "Assigned"
    }, o.assignees.length > 0 ? /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        gap: '10px'
      }
    }, o.assignees.map(n => /*#__PURE__*/React.createElement("div", {
      key: n,
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: '9px'
      }
    }, /*#__PURE__*/React.createElement(Avatar, {
      name: n,
      size: "sm"
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--font-body)',
        color: 'var(--text-primary)'
      }
    }, n)))) : /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--font-body)',
        color: 'var(--text-tertiary)'
      }
    }, "No operator assigned yet.")))));
  }
  window.JobTicket = JobTicket;
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/print-shop/JobTicket.jsx", error: String((e && e.message) || e) }); }

// ui_kits/print-shop/Orders.jsx
try { (() => {
/* Orders screen — filterable table */
(function () {
  const {
    Card,
    Badge,
    Button,
    Input,
    Tabs,
    Tag,
    IconButton
  } = window.FrappeDesignSystem_75694f;
  const {
    Ic,
    PageHeader
  } = window.FK;
  const D = window.FrappeData;
  function Orders({
    onOpenOrder,
    onNewOrder
  }) {
    const [filter, setFilter] = React.useState('All');
    const [q, setQ] = React.useState('');
    const filtered = D.orders.filter(o => {
      const matchQ = !q || (o.customer + o.job + o.id).toLowerCase().includes(q.toLowerCase());
      const matchF = filter === 'All' ? true : filter === 'Open' ? o.status !== 'shipped' : filter === 'Rush' ? o.rush : filter === 'Overdue' ? o.status === 'overdue' : filter === 'Shipped' ? o.status === 'shipped' : true;
      return matchQ && matchF;
    });
    const th = (label, align) => /*#__PURE__*/React.createElement("th", {
      style: {
        textAlign: align || 'left',
        font: 'var(--font-caption)',
        fontWeight: 600,
        textTransform: 'uppercase',
        letterSpacing: 'var(--tracking-caps)',
        color: 'var(--text-tertiary)',
        padding: '9px 16px',
        borderBottom: '1px solid var(--border-subtle)',
        whiteSpace: 'nowrap'
      }
    }, label);
    const td = (children, extra) => /*#__PURE__*/React.createElement("td", {
      style: {
        padding: '12px 16px',
        borderBottom: '1px solid var(--border-subtle)',
        font: 'var(--font-body)',
        color: 'var(--text-secondary)',
        ...extra
      }
    }, children);
    return /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement(PageHeader, {
      title: "Orders",
      subtitle: `${D.orders.length} total · ${D.orders.filter(o => o.status !== 'shipped').length} open`,
      actions: /*#__PURE__*/React.createElement(Button, {
        variant: "primary",
        iconLeft: /*#__PURE__*/React.createElement(Ic, {
          n: "Plus",
          size: 15
        }),
        onClick: onNewOrder
      }, "New order")
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: '12px',
        marginBottom: '14px',
        flexWrap: 'wrap'
      }
    }, /*#__PURE__*/React.createElement(Tabs, {
      variant: "pill",
      value: filter,
      onChange: setFilter,
      tabs: ['All', 'Open', 'Rush', 'Overdue', 'Shipped']
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        width: '240px'
      }
    }, /*#__PURE__*/React.createElement(Input, {
      placeholder: "Search orders\u2026",
      value: q,
      onChange: e => setQ(e.target.value),
      iconLeft: /*#__PURE__*/React.createElement(Ic, {
        n: "Search",
        size: 14
      }),
      size: "sm"
    }))), /*#__PURE__*/React.createElement(Card, {
      padding: "none"
    }, /*#__PURE__*/React.createElement("table", {
      style: {
        width: '100%',
        borderCollapse: 'collapse'
      }
    }, /*#__PURE__*/React.createElement("thead", null, /*#__PURE__*/React.createElement("tr", null, th('Order'), th('Customer'), th('Job'), th('Qty', 'right'), th('Due'), th('Status'), th('Total', 'right'), th(''))), /*#__PURE__*/React.createElement("tbody", null, filtered.map(o => {
      const b = D.statusBadge[o.status];
      return /*#__PURE__*/React.createElement("tr", {
        key: o.id,
        onClick: () => onOpenOrder(o),
        style: {
          cursor: 'pointer'
        },
        onMouseEnter: e => e.currentTarget.style.background = 'var(--surface-hover)',
        onMouseLeave: e => e.currentTarget.style.background = 'transparent'
      }, td(/*#__PURE__*/React.createElement("span", {
        style: {
          display: 'flex',
          alignItems: 'center',
          gap: '7px'
        }
      }, /*#__PURE__*/React.createElement("span", {
        style: {
          fontFamily: 'var(--font-mono)',
          fontWeight: 500,
          color: 'var(--text-primary)'
        },
        className: "tabular"
      }, "#", o.id), o.rush && /*#__PURE__*/React.createElement(Badge, {
        tone: "brand",
        size: "sm"
      }, "Rush"))), td(/*#__PURE__*/React.createElement("span", {
        style: {
          font: 'var(--font-body-strong)',
          color: 'var(--text-primary)'
        }
      }, o.customer)), td(/*#__PURE__*/React.createElement("span", {
        style: {
          display: 'inline-block',
          maxWidth: '200px',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap'
        }
      }, o.job)), td(/*#__PURE__*/React.createElement("span", {
        className: "tabular",
        style: {
          fontFamily: 'var(--font-mono)'
        }
      }, o.qty.toLocaleString()), {
        textAlign: 'right',
        color: 'var(--text-primary)'
      }), td(/*#__PURE__*/React.createElement("span", {
        style: {
          color: o.status === 'overdue' ? 'var(--danger-text)' : 'var(--text-secondary)',
          fontWeight: o.status === 'overdue' ? 600 : 400
        }
      }, o.due)), td(/*#__PURE__*/React.createElement(Badge, {
        tone: b.tone,
        dot: true,
        size: "sm"
      }, b.label)), td(/*#__PURE__*/React.createElement("span", {
        style: {
          fontFamily: 'var(--font-mono)',
          fontWeight: 500,
          color: 'var(--text-primary)'
        },
        className: "tabular"
      }, D.money(o.total)), {
        textAlign: 'right'
      }), td(/*#__PURE__*/React.createElement("span", {
        style: {
          color: 'var(--text-tertiary)'
        }
      }, /*#__PURE__*/React.createElement(Ic, {
        n: "ChevronRight",
        size: 16
      })), {
        width: '32px'
      }));
    }))), filtered.length === 0 && /*#__PURE__*/React.createElement("div", {
      style: {
        padding: '48px',
        textAlign: 'center',
        color: 'var(--text-tertiary)',
        font: 'var(--font-body)'
      }
    }, "No orders match that filter.")));
  }
  window.Orders = Orders;
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/print-shop/Orders.jsx", error: String((e && e.message) || e) }); }

// ui_kits/print-shop/Production.jsx
try { (() => {
/* Production screen — kanban board */
(function () {
  const {
    Card,
    Badge,
    Avatar,
    AvatarGroup,
    Tag
  } = window.FrappeDesignSystem_75694f;
  const {
    Ic,
    PageHeader
  } = window.FK;
  const D = window.FrappeData;
  function JobCard({
    o,
    onOpen
  }) {
    return /*#__PURE__*/React.createElement("div", {
      onClick: () => onOpen(o),
      style: {
        background: 'var(--surface-card)',
        border: '1px solid var(--border-default)',
        borderRadius: 'var(--radius-md)',
        boxShadow: 'var(--shadow-xs)',
        padding: '11px 12px',
        cursor: 'pointer',
        transition: 'box-shadow var(--duration-fast), transform var(--duration-fast)'
      },
      onMouseEnter: e => {
        e.currentTarget.style.boxShadow = 'var(--shadow-md)';
        e.currentTarget.style.transform = 'translateY(-1px)';
      },
      onMouseLeave: e => {
        e.currentTarget.style.boxShadow = 'var(--shadow-xs)';
        e.currentTarget.style.transform = 'none';
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        marginBottom: '7px'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        fontFamily: 'var(--font-mono)',
        fontWeight: 600,
        fontSize: 'var(--text-sm)',
        color: 'var(--text-primary)'
      },
      className: "tabular"
    }, "#", o.id), o.rush && /*#__PURE__*/React.createElement(Badge, {
      tone: "brand",
      size: "sm"
    }, "Rush")), /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--font-body-strong)',
        color: 'var(--text-primary)',
        marginBottom: '3px',
        lineHeight: 1.3
      }
    }, o.job), /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--font-caption)',
        color: 'var(--text-tertiary)',
        marginBottom: '10px'
      }
    }, o.customer, " \xB7 ", o.qty.toLocaleString(), " \xB7 ", o.stock), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: '5px',
        font: 'var(--font-caption)',
        color: o.status === 'overdue' ? 'var(--danger-text)' : 'var(--text-secondary)'
      }
    }, /*#__PURE__*/React.createElement(Ic, {
      n: "Clock",
      size: 12
    }), o.due), o.assignees.length > 0 ? /*#__PURE__*/React.createElement(AvatarGroup, {
      names: o.assignees,
      max: 2,
      size: "xs"
    }) : /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--font-caption)',
        color: 'var(--text-disabled)'
      }
    }, "Unassigned")));
  }
  function Production({
    onOpenOrder
  }) {
    return /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement(PageHeader, {
      title: "Production",
      subtitle: "Drag jobs across the floor \u2014 Queued through Shipped"
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'grid',
        gridTemplateColumns: `repeat(${D.columns.length}, 1fr)`,
        gap: '12px',
        alignItems: 'start'
      }
    }, D.columns.map(c => {
      const jobs = D.orders.filter(o => o.stage === c.key);
      return /*#__PURE__*/React.createElement("div", {
        key: c.key,
        style: {
          background: 'var(--bg-subtle)',
          border: '1px solid var(--border-subtle)',
          borderRadius: 'var(--radius-lg)',
          padding: '10px',
          minHeight: '120px'
        }
      }, /*#__PURE__*/React.createElement("div", {
        style: {
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '2px 4px 10px'
        }
      }, /*#__PURE__*/React.createElement(Badge, {
        tone: c.tone,
        dot: true,
        size: "sm"
      }, c.key), /*#__PURE__*/React.createElement("span", {
        style: {
          fontFamily: 'var(--font-mono)',
          fontWeight: 600,
          fontSize: 'var(--text-xs)',
          color: 'var(--text-tertiary)'
        },
        className: "tabular"
      }, jobs.length)), /*#__PURE__*/React.createElement("div", {
        style: {
          display: 'flex',
          flexDirection: 'column',
          gap: '9px'
        }
      }, jobs.map(o => /*#__PURE__*/React.createElement(JobCard, {
        key: o.id,
        o: o,
        onOpen: onOpenOrder
      })), jobs.length === 0 && /*#__PURE__*/React.createElement("div", {
        style: {
          padding: '16px 8px',
          textAlign: 'center',
          font: 'var(--font-caption)',
          color: 'var(--text-disabled)'
        }
      }, "Empty")));
    })));
  }
  window.Production = Production;
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/print-shop/Production.jsx", error: String((e && e.message) || e) }); }

// ui_kits/print-shop/Welcome.jsx
try { (() => {
/* Welcome / sign-in + business onboarding (roadmap #5) */
(function () {
  const {
    Button,
    Input,
    Select
  } = window.FrappeDesignSystem_75694f;
  const {
    Ic
  } = window.FK;
  function Welcome({
    onEnter
  }) {
    const [step, setStep] = React.useState('signin'); // signin | onboard

    return /*#__PURE__*/React.createElement("div", {
      style: {
        height: '100vh',
        width: '100vw',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'var(--bg-base)',
        padding: '24px',
        boxSizing: 'border-box'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        width: '380px',
        maxWidth: '100%'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        marginBottom: '24px'
      }
    }, /*#__PURE__*/React.createElement("img", {
      src: "../../assets/frappe-logo.svg",
      width: "44",
      height: "42",
      alt: "Frappe"
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        marginTop: '14px',
        fontFamily: 'var(--font-sans)',
        fontWeight: 600,
        fontSize: 'var(--text-2xl)',
        lineHeight: 1.25,
        color: 'var(--text-primary)',
        letterSpacing: 'var(--tracking-tight)',
        textAlign: 'center',
        whiteSpace: 'nowrap'
      }
    }, step === 'signin' ? 'Welcome to Frappe' : 'Tell us about your shop'), /*#__PURE__*/React.createElement("div", {
      style: {
        marginTop: '6px',
        fontFamily: 'var(--font-sans)',
        fontWeight: 400,
        fontSize: 'var(--text-md)',
        lineHeight: 1.5,
        color: 'var(--text-secondary)',
        textAlign: 'center'
      }
    }, step === 'signin' ? 'Run your print shop — quotes to shipping.' : 'We’ll tailor your workspace.')), /*#__PURE__*/React.createElement("div", {
      style: {
        background: 'var(--surface-card)',
        border: '1px solid var(--border-default)',
        borderRadius: 'var(--radius-lg)',
        boxShadow: 'var(--shadow-md)',
        padding: '22px'
      }
    }, step === 'signin' ? /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        gap: '12px'
      }
    }, /*#__PURE__*/React.createElement(Button, {
      variant: "secondary",
      fullWidth: true,
      iconLeft: /*#__PURE__*/React.createElement(Ic, {
        n: "Mail",
        size: 16
      }),
      onClick: () => setStep('onboard')
    }, "Continue with email"), /*#__PURE__*/React.createElement(Button, {
      variant: "secondary",
      fullWidth: true,
      onClick: () => setStep('onboard')
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'inline-flex',
        marginRight: '8px'
      }
    }, /*#__PURE__*/React.createElement("svg", {
      width: "16",
      height: "16"
    }, /*#__PURE__*/React.createElement("use", {
      href: "../../assets/social-icons.svg#google-sheets-icon"
    }))), "Continue with Google"), /*#__PURE__*/React.createElement("div", {
      style: {
        textAlign: 'center',
        font: 'var(--font-caption)',
        color: 'var(--text-tertiary)',
        margin: '4px 0'
      }
    }, "Choose how you\u2019d like to sign in.")) : /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        gap: '14px'
      }
    }, /*#__PURE__*/React.createElement(Input, {
      label: "Shop name",
      placeholder: "Bowen Print Co.",
      defaultValue: "Bowen Print Co."
    }), /*#__PURE__*/React.createElement(Select, {
      label: "What do you print most?",
      options: ['Business cards & stationery', 'Large format & signage', 'Wedding & events', 'Apparel & promo', 'A bit of everything']
    }), /*#__PURE__*/React.createElement(Select, {
      label: "Team size",
      options: ['Just me', '2–5', '6–15', '16+']
    }), /*#__PURE__*/React.createElement(Button, {
      variant: "primary",
      fullWidth: true,
      iconRight: /*#__PURE__*/React.createElement(Ic, {
        n: "ArrowRight",
        size: 16
      }),
      onClick: onEnter
    }, "Open my workspace"))), /*#__PURE__*/React.createElement("div", {
      style: {
        textAlign: 'center',
        marginTop: '16px',
        font: 'var(--font-caption)',
        color: 'var(--text-tertiary)'
      }
    }, "Local-first \xB7 your data stays on this machine")));
  }
  window.Welcome = Welcome;
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/print-shop/Welcome.jsx", error: String((e && e.message) || e) }); }

// ui_kits/print-shop/data.js
try { (() => {
// Fake print-shop data for the Frappe UI kit. Plain globals (no modules).
window.FrappeData = function () {
  const orders = [{
    id: 1048,
    customer: 'Acme Co.',
    contact: 'Dana Ruiz',
    job: '500 matte business cards',
    qty: 500,
    stock: '16pt Matte',
    status: 'press',
    total: 184.00,
    due: 'Today, 4:00pm',
    rush: true,
    stage: 'On press',
    assignees: ['Max Bowen']
  }, {
    id: 1047,
    customer: 'Northwind Cafe',
    contact: 'Priya Shah',
    job: 'A-frame sidewalk banner',
    qty: 2,
    stock: '13oz Vinyl',
    status: 'art',
    total: 240.00,
    due: 'Tomorrow',
    rush: false,
    stage: 'Awaiting art',
    assignees: ['Dana Ruiz']
  }, {
    id: 1046,
    customer: 'Lumen Studio',
    contact: 'Lee Ortiz',
    job: 'Tri-fold brochures',
    qty: 1000,
    stock: '100lb Gloss',
    status: 'queued',
    total: 612.50,
    due: 'Fri',
    rush: false,
    stage: 'Queued',
    assignees: []
  }, {
    id: 1045,
    customer: 'Harbor Yoga',
    contact: 'Sam Kade',
    job: 'Vinyl window decals',
    qty: 12,
    stock: 'Cut Vinyl',
    status: 'queued',
    total: 96.00,
    due: 'Fri',
    rush: false,
    stage: 'Prepress',
    assignees: ['Priya Shah']
  }, {
    id: 1044,
    customer: 'Acme Co.',
    contact: 'Dana Ruiz',
    job: 'Letterhead reprint',
    qty: 250,
    stock: '70lb Uncoated',
    status: 'shipped',
    total: 78.00,
    due: 'Shipped',
    rush: false,
    stage: 'Shipped',
    assignees: ['Max Bowen']
  }, {
    id: 1043,
    customer: 'Pine & Oak',
    contact: 'Jo Tran',
    job: 'Foil wedding invites',
    qty: 120,
    stock: '120lb Cotton',
    status: 'press',
    total: 540.00,
    due: 'Mon',
    rush: false,
    stage: 'Bindery',
    assignees: ['Lee Ortiz', 'Sam Kade']
  }, {
    id: 1042,
    customer: 'Bright Labs',
    contact: 'Avery Cole',
    job: 'Roll-up retractable banner',
    qty: 1,
    stock: 'Polyester',
    status: 'overdue',
    total: 159.00,
    due: 'Overdue 1d',
    rush: true,
    stage: 'On press',
    assignees: ['Max Bowen']
  }, {
    id: 1041,
    customer: 'Cedar Dental',
    contact: 'Rae Kim',
    job: 'Appointment cards',
    qty: 2000,
    stock: '14pt Gloss',
    status: 'queued',
    total: 220.00,
    due: 'Next Tue',
    rush: false,
    stage: 'Queued',
    assignees: []
  }];
  const invoices = [{
    id: 'INV-2048',
    customer: 'Acme Co.',
    order: 1048,
    amount: 184.00,
    status: 'sent',
    date: 'Jun 18',
    deposit: 50
  }, {
    id: 'INV-2047',
    customer: 'Northwind Cafe',
    order: 1047,
    amount: 240.00,
    status: 'paid',
    date: 'Jun 17',
    deposit: 240
  }, {
    id: 'INV-2046',
    customer: 'Bright Labs',
    order: 1042,
    amount: 159.00,
    status: 'overdue',
    date: 'Jun 10',
    deposit: 0
  }, {
    id: 'INV-2045',
    customer: 'Pine & Oak',
    order: 1043,
    amount: 540.00,
    status: 'deposit',
    date: 'Jun 15',
    deposit: 270
  }, {
    id: 'INV-2044',
    customer: 'Lumen Studio',
    order: 1046,
    amount: 612.50,
    status: 'draft',
    date: '—',
    deposit: 0
  }, {
    id: 'INV-2043',
    customer: 'Acme Co.',
    order: 1044,
    amount: 78.00,
    status: 'paid',
    date: 'Jun 12',
    deposit: 78
  }];

  // Kanban columns for production
  const columns = [{
    key: 'Queued',
    tone: 'info'
  }, {
    key: 'Prepress',
    tone: 'brand'
  }, {
    key: 'On press',
    tone: 'brand'
  }, {
    key: 'Bindery',
    tone: 'warning'
  }, {
    key: 'Shipped',
    tone: 'success'
  }];
  const statusBadge = {
    queued: {
      tone: 'info',
      label: 'Queued'
    },
    art: {
      tone: 'warning',
      label: 'Awaiting art'
    },
    press: {
      tone: 'brand',
      label: 'On press'
    },
    shipped: {
      tone: 'success',
      label: 'Shipped'
    },
    overdue: {
      tone: 'danger',
      label: 'Overdue'
    }
  };
  const invoiceBadge = {
    draft: {
      tone: 'neutral',
      label: 'Draft'
    },
    sent: {
      tone: 'info',
      label: 'Sent'
    },
    paid: {
      tone: 'success',
      label: 'Paid'
    },
    overdue: {
      tone: 'danger',
      label: 'Overdue'
    },
    deposit: {
      tone: 'warning',
      label: 'Deposit paid'
    }
  };
  const money = n => '$' + n.toLocaleString('en-US', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  });
  return {
    orders,
    invoices,
    columns,
    statusBadge,
    invoiceBadge,
    money
  };
}();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/print-shop/data.js", error: String((e && e.message) || e) }); }

// ui_kits/print-shop/shared.jsx
try { (() => {
/* Shared atoms for the print-shop kit. Exposes window.FK = { Ic, ... } */
(function () {
  const {
    useState
  } = React;

  // Lucide icon → React, themed via currentColor
  function Ic({
    n,
    size = 16,
    style
  }) {
    const node = window.lucide && window.lucide[n];
    if (!node) return null;
    const svg = window.lucide.createElement(node);
    svg.setAttribute('width', size);
    svg.setAttribute('height', size);
    return /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'inline-flex',
        ...style
      },
      dangerouslySetInnerHTML: {
        __html: svg.outerHTML
      }
    });
  }

  // Tiny uppercase eyebrow label
  function Eyebrow({
    children,
    style
  }) {
    return /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--font-caption)',
        fontWeight: 600,
        textTransform: 'uppercase',
        letterSpacing: 'var(--tracking-caps)',
        color: 'var(--text-tertiary)',
        ...style
      }
    }, children);
  }

  // KPI stat card for the dashboard
  function Kpi({
    label,
    value,
    delta,
    deltaTone = 'success',
    icon
  }) {
    return /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1,
        minWidth: 0,
        background: 'var(--surface-card)',
        border: '1px solid var(--border-default)',
        borderRadius: 'var(--radius-lg)',
        boxShadow: 'var(--shadow-sm)',
        padding: '16px 18px'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        marginBottom: '10px'
      }
    }, /*#__PURE__*/React.createElement(Eyebrow, null, label), /*#__PURE__*/React.createElement("span", {
      style: {
        color: 'var(--text-tertiary)'
      }
    }, /*#__PURE__*/React.createElement(Ic, {
      n: icon,
      size: 16
    }))), /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--font-h1)',
        fontFamily: 'var(--font-mono)',
        letterSpacing: '-0.02em',
        color: 'var(--text-primary)'
      },
      className: "tabular"
    }, value), delta && /*#__PURE__*/React.createElement("div", {
      style: {
        marginTop: '6px',
        font: 'var(--font-caption)',
        fontWeight: 500,
        color: deltaTone === 'success' ? 'var(--success-text)' : deltaTone === 'danger' ? 'var(--danger-text)' : 'var(--text-secondary)'
      }
    }, delta));
  }

  // Page title row
  function PageHeader({
    title,
    subtitle,
    actions
  }) {
    return /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'flex-start',
        justifyContent: 'space-between',
        gap: '16px',
        marginBottom: '20px'
      }
    }, /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("h1", {
      style: {
        margin: 0,
        font: 'var(--font-h2)',
        color: 'var(--text-primary)',
        letterSpacing: 'var(--tracking-tight)'
      }
    }, title), subtitle && /*#__PURE__*/React.createElement("div", {
      style: {
        marginTop: '4px',
        font: 'var(--font-body)',
        color: 'var(--text-secondary)'
      }
    }, subtitle)), actions && /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        gap: '8px',
        flexShrink: 0
      }
    }, actions));
  }
  window.FK = {
    Ic,
    Eyebrow,
    Kpi,
    PageHeader
  };
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/print-shop/shared.jsx", error: String((e && e.message) || e) }); }

__ds_ns.Avatar = __ds_scope.Avatar;

__ds_ns.AvatarGroup = __ds_scope.AvatarGroup;

__ds_ns.Badge = __ds_scope.Badge;

__ds_ns.Card = __ds_scope.Card;

__ds_ns.Tag = __ds_scope.Tag;

__ds_ns.Dialog = __ds_scope.Dialog;

__ds_ns.Toast = __ds_scope.Toast;

__ds_ns.ToastViewport = __ds_scope.ToastViewport;

__ds_ns.Tooltip = __ds_scope.Tooltip;

__ds_ns.Button = __ds_scope.Button;

__ds_ns.Checkbox = __ds_scope.Checkbox;

__ds_ns.IconButton = __ds_scope.IconButton;

__ds_ns.Input = __ds_scope.Input;

__ds_ns.Select = __ds_scope.Select;

__ds_ns.Switch = __ds_scope.Switch;

__ds_ns.Tabs = __ds_scope.Tabs;

})();
