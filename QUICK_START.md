# Frappe Design System - Quick Start (30 seconds)

## What You Got
A complete, production-ready design system with:
- **16 React components** ready to use immediately
- **Comprehensive design tokens** (colors, type, spacing, motion)
- **Light & dark mode** (automatic theme switching)
- **Full TypeScript support**

## Use It Right Now

### 1. Import a Component
```tsx
import { Button, Input, Badge } from 'src/design-system'

export function MyForm() {
  return (
    <Button variant="primary">
      Click me
    </Button>
  )
}
```

### 2. Use Design Tokens in CSS
```css
.my-component {
  color: var(--text-primary);
  background: var(--surface-card);
  padding: var(--space-4);
}
```

### 3. Toggle Dark Mode
```tsx
// Light (default)
document.documentElement.setAttribute('data-theme', 'light')

// Dark
document.documentElement.setAttribute('data-theme', 'dark')
```

## 16 Components Available

| Category | Components |
|----------|------------|
| **Forms** | Button, IconButton, Input, Select, Checkbox, Switch |
| **Display** | Badge, Card, Tag, Avatar, AvatarGroup |
| **Navigation** | Tabs |
| **Feedback** | Dialog, Toast, Tooltip |

## Common Tokens

```css
/* Colors */
--brand              /* Primary violet (#863bff) */
--text-primary       /* Dark gray in light mode, light gray in dark */
--surface-card       /* White in light, dark navy in dark */
--border-default     /* Light gray in light, dark gray in dark */
--success            /* Green status */
--warning            /* Amber status */
--danger             /* Red status */

/* Typography */
--font-sans          /* Geist (app UI) */
--font-mono          /* Geist Mono (data/numbers) */
--font-body          /* 14px regular text */
--font-h1, --font-h2 /* Headings */

/* Spacing (4px grid) */
--space-4            /* 8px */
--space-8            /* 16px */
--space-12           /* 24px */
--space-16           /* 32px */

/* Other */
--radius-md          /* 8px rounded corners */
--radius-lg          /* 12px for cards */
--shadow-sm          /* Subtle shadow */
--duration-base      /* 200ms for transitions */
```

## Where Everything Is

| File | Purpose |
|------|---------|
| `src/design-system/index.ts` | Component exports |
| `src/design-system/styles.css` | CSS entry point |
| `src/design-system/tokens/` | Design tokens (colors, type, spacing) |
| `src/design-system/components/` | React component source files |
| `src/design-system/INTEGRATION_GUIDE.md` | Full usage guide |
| `src/design-system/readme.md` | Complete design spec |
| `src/components/DesignSystemExample.tsx` | Live examples |

## See It In Action

1. Open `src/components/DesignSystemExample.tsx` to see all components in use
2. Update `src/App.tsx` to import and render the example component
3. Run `npm run dev` and visit the app
4. Toggle dark mode with browser dev tools: `document.documentElement.setAttribute('data-theme', 'dark')`

## Reading Order

1. **This file** (you are here) — 30-second overview
2. **DESIGN_SYSTEM_SETUP.md** — What was installed & next steps
3. **src/design-system/INTEGRATION_GUIDE.md** — How to use everything
4. **src/design-system/readme.md** — Deep dive on design system philosophy
5. **src/components/DesignSystemExample.tsx** — Working code examples

## Common Tasks

### Add a button to a form
```tsx
import { Button } from 'src/design-system'

<Button variant="primary" size="md" onClick={handleSave}>
  Save changes
</Button>
```

### Create a styled card
```tsx
import { Card } from 'src/design-system'

<Card style={{ padding: 'var(--space-16)' }}>
  <h2>My Card</h2>
  <p>Card content here</p>
</Card>
```

### Show a status badge
```tsx
import { Badge } from 'src/design-system'

<Badge tone="success">Approved</Badge>
<Badge tone="warning">Pending</Badge>
<Badge tone="danger">Rejected</Badge>
```

### Use a form input
```tsx
import { Input } from 'src/design-system'

<Input 
  label="Order ID"
  placeholder="e.g., INV-1234"
  type="text"
/>
```

### Show a confirmation dialog
```tsx
import { Dialog, Button } from 'src/design-system'

<Dialog 
  open={isOpen} 
  onOpenChange={setIsOpen}
  title="Delete order?"
>
  <p>This cannot be undone.</p>
  <Button variant="danger" onClick={handleDelete}>
    Delete
  </Button>
</Dialog>
```

## Icons

Use **Lucide icons** — install with `npm install lucide-react`

```tsx
import { ChevronDown, Plus, Trash2 } from 'lucide-react'

<IconButton>
  <Plus size={16} />
</IconButton>
```

## Content Voice

Write like a shop-floor colleague:
- ✅ "New order", "Send invoice", "Approve proof"
- ❌ Don't use: "Supercharge your workflow", "Let's get started!"
- ✅ Domain words: jobs, orders, proofs, docket, quote, estimate
- ❌ Generic SaaS speak

## Need Help?

- **"How do I use X component?"** → Check `src/design-system/components/[category]/[Component].prompt.md`
- **"What tokens are available?"** → Check `src/design-system/tokens/colors.css` and `spacing.css`
- **"How do I write copy?"** → Read "Content fundamentals" in `src/design-system/readme.md`
- **"Can I customize colors?"** → Yes! Override CSS custom properties in your CSS

---

## TL;DR

```tsx
// Import components from src/design-system
import { Button, Input, Badge, Dialog } from 'src/design-system'

// Use design tokens for styling
style={{ color: 'var(--text-primary)', padding: 'var(--space-4)' }}

// Everything has light & dark mode built-in
document.documentElement.setAttribute('data-theme', 'dark')
```

**That's it! You're ready to build.** 🚀

Read `DESIGN_SYSTEM_SETUP.md` for details and next steps.
