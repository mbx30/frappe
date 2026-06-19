# Frappe Design System Integration Guide

## Overview
The Frappe Design System is now integrated into your Vite + React project. All design tokens, components, and styles are available for immediate use.

## File Structure
```
src/design-system/
├── styles.css                 # Main entry point for all design tokens
├── tokens/
│   ├── colors.css            # Color tokens (light & dark themes)
│   ├── typography.css        # Font families and type scale
│   ├── spacing.css           # Spacing, radii, elevation, motion
│   └── fonts.css             # Webfont imports (Geist, Geist Mono)
├── components/
│   ├── forms/                # Button, Input, Select, Checkbox, Switch, IconButton
│   ├── display/              # Badge, Card, Tag, Avatar, AvatarGroup
│   ├── feedback/             # Dialog, Toast, Tooltip
│   └── navigation/           # Tabs
├── assets/
│   ├── frappe-logo.svg       # Brand mark (violet lightning)
│   └── social-icons.svg      # Third-party brand icons
├── guidelines/               # Design foundation specimens (colors, type, spacing)
├── ui_kits/                  # Full-screen interactive app layouts
├── readme.md                 # Complete design system documentation
└── index.ts                  # TypeScript exports (NEW)
```

## Quick Start

### 1. Import Components
```tsx
import { Button, Input, Badge, Dialog } from 'src/design-system'

export function MyComponent() {
  return (
    <Button variant="primary" size="md">
      Click me
    </Button>
  )
}
```

### 2. Use Design Tokens
All CSS tokens are automatically available. They flip automatically between light/dark themes via `[data-theme="dark"]` on the root element.

```css
.my-component {
  color: var(--text-primary);
  background: var(--surface-card);
  border: 1px solid var(--border-default);
  padding: var(--space-4);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-sm);
}
```

### 3. Enable Dark Mode
Add `data-theme="dark"` to your `<html>` element to switch themes:

```tsx
document.documentElement.setAttribute('data-theme', 'dark')
```

## Design Tokens Reference

### Colors
- **Brand**: `--brand` (violet #863bff), `--brand-hover`, `--brand-active`, `--brand-subtle`
- **Surfaces**: `--surface-card`, `--surface-raised`, `--surface-hover`, `--surface-active`, `--surface-inset`
- **Text**: `--text-primary`, `--text-secondary`, `--text-tertiary`, `--text-disabled`
- **Borders**: `--border-subtle`, `--border-default`, `--border-strong`
- **Status**: `--success`, `--warning`, `--danger`, `--info` (with `-subtle` and `-text` variants)

### Typography
- **Fonts**: `--font-sans` (Geist), `--font-mono` (Geist Mono)
- **Scale**: `--text-2xs` (11px) → `--text-5xl` (48px)
- **Semantic**: `--font-display`, `--font-h1`, `--font-h2`, `--font-h3`, `--font-title`, `--font-body`, `--font-label`, `--font-caption`

### Spacing
- **Scale**: `--space-0` → `--space-40` (0–80px on 4px grid)
- **Radii**: `--radius-xs` (3px) → `--radius-full` (999px)
- **Chrome**: `--sidebar-width` (248px), `--topbar-height` (52px), `--row-height` (38px)
- **Motion**: `--duration-instant` (80ms) → `--duration-slow` (320ms), easing via `--ease-standard`

## Component Library

### Form Controls
- **Button** — Primary, secondary, tertiary, danger variants; sm, md, lg sizes
- **IconButton** — Compact icon-only buttons
- **Input** — Text input with labels, icons, mono numbers, error states
- **Select** — Dropdown with optional searchability
- **Checkbox** — Accessible checkboxes with labels
- **Switch** — Toggle switches

### Display
- **Badge** — Status pills (success, warning, danger, info)
- **Card** — Panels with borders, shadows, and semantic tokens
- **Tag** — Filter chips with optional counts
- **Avatar / AvatarGroup** — User identity with images or initials

### Navigation
- **Tabs** — Underline (page-level) and pill (sub-filter) variants

### Feedback
- **Dialog** — Modal with backdrop scrim, optional footer actions
- **Toast** — Transient notifications; position with ToastViewport
- **Tooltip** — Hover-triggered inline help text

## Content Voice & Vocabulary

Write like a shop-floor colleague:
- **Sentence case everywhere** (not Title Case)
- **Imperative verbs for buttons**: "New order", "Send invoice", "Approve proof"
- **Domain vocabulary**: Jobs, orders, proofs, prepress, art approval, docket, estimate, quote, invoice
- **No emoji in product UI** (use Lucide icons instead)
- **Numbers are exact**: `$1,240.00`, `500 units`, `#1042` in monospace

## Iconography
- Use **Lucide** icons from `https://lucide.dev` — install with `npm install lucide-react`
- Import: `import { ChevronDown } from 'lucide-react'`
- Use semantic tokens for colors so icons adapt to theme: `<Icon className="text-[var(--text-primary)]" />`

## Layout Patterns

### App Chrome
- **Fixed left sidebar**: 248px wide, workspace navigation
- **Slim top bar**: 52px tall for account/menu
- **Tab navigation**: Underline for page-level tabs, pill for sub-filters
- **Content scrolls**, chrome stays put

### Table/Grid
- Fixed row height (38px or 32px for compact)
- Tabular figures for numbers (mono font with `font-variant-numeric: tabular-nums`)
- Toolbar above, not below

## Theming
The system includes first-class light AND dark mode. Both themes are automatically managed:
- Light mode is **default** (no attribute needed)
- Dark mode is enabled by adding `data-theme="dark"` to `<html>`
- All tokens flip automatically via CSS custom properties
- Consumers use semantic tokens (`--text-primary`, `--surface-card`) so both themes work for free

## Motion & Interaction
- **Durations**: `--duration-instant` (80ms) for snappy state, `--duration-base` (200ms) for standard
- **Easing**: `--ease-standard` (cubic-bezier(0.2,0,0,1)) for entrance/exit, never bouncy
- **Hover**: Surfaces darken/lighten one step; text stays put
- **Press**: Buttons scale down (0.97) for tactile feedback
- **Focus**: 3px violet ring + brand-colored border, always visible
- **Respect `prefers-reduced-motion`**: Drop transforms, keep instant state changes

## Build & Performance
- **Bundle size**: Design system CSS is ~25KB (uncompressed), easily tree-shakeable
- **Fonts**: Geist and Geist Mono are loaded via `@import` in `tokens/fonts.css`
- **No JS overhead**: Tokens are pure CSS; components are React with no extra dependencies
- **Vite integration**: Works out-of-the-box with Vite's CSS module support

## Migrating Existing Components
To use design system components in place of custom ones:

1. Audit existing components for functional duplicates (e.g., custom Button → design Button)
2. Identify styling differences and map to new tokens
3. Update imports: `import Button from './components/Button'` → `import { Button } from 'src/design-system'`
4. Test behavior & appearance in light and dark modes
5. Remove old component files once migrated

## Support & Documentation
- Full **readme.md** in this directory covers all visual foundations, voice, icons, layout patterns
- Component `.d.ts` files list all props and defaults
- Component `.prompt.md` files show usage examples
- Guidelines HTML cards demonstrate tokens visually
- UI kit shows complete app layouts in context

## Example: Building a Form
```tsx
import { 
  Button, 
  Input, 
  Select, 
  Checkbox, 
  Dialog 
} from 'src/design-system'

export function OrderForm() {
  const [open, setOpen] = useState(false)
  
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <form style={{ display: 'grid', gap: 'var(--space-4)' }}>
        <Input 
          label="Order ID" 
          placeholder="INV-1234"
          type="text"
        />
        <Select 
          label="Status"
          options={[
            { value: 'draft', label: 'Draft' },
            { value: 'queued', label: 'Queued' },
            { value: 'approved', label: 'Approved' }
          ]}
        />
        <Checkbox label="Rush shipping" />
        <Button variant="primary">Submit order</Button>
      </form>
    </Dialog>
  )
}
```

---

For detailed information on design tokens, components, accessibility, and layout patterns, see `readme.md`.
