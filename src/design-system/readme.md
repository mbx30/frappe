# Frappe Design System

**Frappe** is a local-first, cross-platform desktop application for **print shops** — a print MIS (Management Information System). It began as a fast spreadsheet/workbook tool (Tauri + React + `react-data-grid`, with CSV / Excel / Google Sheets / Notion import) and is growing into a full shop-floor platform: estimating & quoting, order management, prepress & art approval, production (kanban + job tickets), shipping & pickup, invoicing, deposits & POS payments, inventory, and QuickBooks sync.

The product is **fast, local, and utilitarian** — built for people quoting jobs and pushing work across a shop floor, not a marketing site. The design system reflects that: tight density, a clear hierarchy, tab-driven navigation, and a confident violet brand over neutral grays, in both light and dark.

This folder is the single source of truth for building Frappe-branded interfaces — production UI or throwaway mocks. Link `styles.css`, mount components from the bundle, and lift screens from the UI kit.

---

## Sources

These inputs informed this system. The reader may not have access; they are recorded for provenance and deeper exploration.

- **Codebase** — the Frappe Tauri/React app (attached locally as `frappe/`). Key reads: `src/index.css` (original CSS variables), `src/App.tsx` / `App.css` (sidebar + tab layout), `src/components/*` (Toolbar, WorkbookList, Spreadsheet, CloudImportDialog), `src/types.ts`, `public/favicon.svg` (the brand mark), `public/icons.svg` (import-source brand icons), `src-tauri/tauri.conf.json`.
- **GitHub** — <https://github.com/mbx30/frappe>. Browse the repo to see the live app code, and the **Issues** (#1–#18) which define the product roadmap: onboarding & OAuth sign-in, estimating & quoting, order management, prepress & art approval, production management, dashboard & kanban views, job ticket/docket, shipping/split-quantity/pickup, invoicing, deposits & partial payments, automated invoice reminders, in-person & POS payments, inventory management, and QuickBooks integration. **Explore this repository to build more accurately against the real product.**

> The brand mark in `assets/frappe-logo.svg` (violet `#863bff` with a `#47bfff` blue accent) is the real app favicon. The original code's `#646cff` accent was Vite scaffolding boilerplate — the violet mark is the true brand, and the token system is anchored on it.

---

## Content fundamentals

How Frappe writes copy. Match this in any UI or asset.

- **Voice: plain, operational, shop-floor.** Frappe talks like a colleague at the counter, not a SaaS brand. Short, concrete, task-first. No hype, no exclamation marks, no "Supercharge your workflow."
- **Address the user as "you"; the product is implicit.** "Email proof to customer", "This can't be undone." Avoid "we" except in genuine system voice ("We couldn't reach QuickBooks").
- **Sentence case everywhere** — buttons, headings, labels, menus. Never Title Case ("New order", not "New Order"). UPPERCASE is reserved for tiny eyebrow/section labels with letter-spacing (`--tracking-caps`).
- **Domain vocabulary is print-shop, not generic SaaS.** Jobs, orders, proofs, prepress, art approval, stock/paper, imposition, press, bindery, quote, estimate, docket/job ticket, deposit, pickup. Use the trade's words.
- **Buttons are imperative verbs:** "New order", "Send invoice", "Approve proof", "Record payment", "Split quantity". Destructive actions name the consequence: "Delete workbook", not "OK".
- **Numbers are first-class and exact.** Money, quantities, IDs and dates render in mono with tabular figures: `$1,240.00`, `#1042`, `INV-2048`, `500 units`. Always show currency and units.
- **Status language is consistent:** Queued · On press · Awaiting art · Shipped · Overdue · Paid · Draft. (See Badge tones.)
- **Empty & error states are direct and actionable.** "No orders yet. Create your first order to get started." / "That address is already in use." Say what happened and the next step.
- **No emoji** in product UI. Meaning is carried by icons (Lucide) and color, never emoji.

---

## Visual foundations

The motifs and rules that make an interface read as Frappe.

### Color & theme
- **Brand is a single confident violet** (`--brand` `#863bff`), used sparingly: primary buttons, active tab underline, focus rings, selected rows, key links. It is an accent, not a wash — most of the UI is neutral. A **blue** (`#47bfff`) from the logo is a secondary accent, used lightly (info, links in dark mode).
- **Neutrals carry the UI.** A cool-cast gray ramp (`--neutral-*`) builds surfaces, borders and text. Backgrounds are flat — **no gradients, no purple washes, no decorative imagery** behind content.
- **First-class light AND dark.** Every surface/text/border/status token flips via `[data-theme="dark"]` on a root element. Dark is a true near-black (`#0d0d10` canvas, `#17171c` cards) with the same violet brand. Default (no attribute) is light. Build with the **semantic** tokens (`--surface-card`, `--text-primary`, `--border-default`, `--brand`) so both themes work for free.
- **Status = subtle fill + readable text + matching dot/border**, never just color: success (green), warning (amber), danger (red), info (sky). See `--success`/`--success-subtle`/`--success-text` triples.

### Type
- **Geist** for all UI; **Geist Mono** for any figure that should align — money, quantities, IDs, dates, table numerics (`font-variant-numeric: tabular-nums`). This mono-for-data split is a signature of the system.
- Compact scale (base UI text **14px**), tight tracking on headings (`--tracking-tight`). Headings are semibold (600), not heavy. Tiny uppercase eyebrow labels use `--tracking-caps`.

### Space, shape, elevation
- **4px grid** (`--space-*`). Dense by intent — this is a working tool with tables, rows and toolbars, not an airy landing page.
- **Modest radii:** controls/inputs `--radius-sm`–`md` (5–8px), cards `--radius-lg` (12px), pills `--radius-full`. Nothing is a blobby super-ellipse.
- **Cards** = `--surface-card` fill, **1px `--border-default`**, `--radius-lg`, and a **low, cool shadow** (`--shadow-sm`). Borders do the structural work; shadow is a whisper. **No rounded-corner-with-colored-left-border cards.**
- **Shadow ramp** (`--shadow-xs`→`lg`) is low-spread and slightly cool; dark mode deepens it. A dedicated `--shadow-brand` (violet glow) is reserved for primary emphasis only.

### Motion & interaction
- **Quick and precise — apps should feel instant.** Durations 80–320ms (`--duration-*`), standard easing `cubic-bezier(0.2,0,0,1)`. **No bounce, no springy overshoot.**
- **Hover:** surfaces go one step darker/lighter (`--surface-hover`); brand fills lighten (`--brand-hover`). Subtle, never a big jump.
- **Press:** buttons **scale down** (0.97; IconButton 0.94) — a tactile "click", not a color flash.
- **Focus:** a **3px violet ring** (`--focus-ring`) plus a brand-colored border. Always visible for keyboard users.
- **Transitions are on color/transform/box-shadow**, not layout. Dialogs pop in (scale + fade, no bounce); scrim blurs the canvas.
- Honor `prefers-reduced-motion`: drop transforms, keep instant state changes.

### Layout
- **App chrome:** fixed left **sidebar** (`--sidebar-width` 248px) for workspace/workbook navigation, a slim **top bar** (`--topbar-height` 52px), and **tabs** as the primary view switcher (underline for top-level pages, pill for sub-filters). Content scrolls; chrome stays put.
- Tables/grids use fixed row heights (`--row-height`) and tabular figures; toolbars sit above them.
- Transparency/blur is used **only** for overlays (modal scrim) — never decoratively over content.

---

## Iconography

- **Lucide** (<https://lucide.dev>) is the icon system — a clean, consistent **outline** set (2px stroke, rounded caps) that matches Geist's neutral-modern tone. Use it for all UI icons. In these specimens it's loaded from CDN (`unpkg.com/lucide`); in production install `lucide-react`. Typical sizes: **14–16px** inline/in controls, 18–20px for nav.
- The codebase's own **import-source brand icons** (Google Sheets, Notion, Bluesky, etc.) live in `assets/social-icons.svg` as an SVG `<symbol>` sprite — copied verbatim from `public/icons.svg`. Reference a glyph with `<svg><use href="assets/social-icons.svg#notion-icon"/></svg>`. Use these only for the literal third-party brands; everything else is Lucide.
- The **Frappe logo** is `assets/frappe-logo.svg` (violet lightning/flow mark). Use it for app identity, splash, and the sidebar header.
- **No emoji** as icons. **No Unicode-glyph icons.** **Do not hand-draw SVG icons** — use Lucide, or the sprite for third-party brands. Color icons with `currentColor` so they inherit text color and theme automatically; use `--brand` only for active/selected states.

---

## What's in this folder

| Path | What |
|------|------|
| `styles.css` | **Entry point.** `@import`s every token + font file. Consumers link only this. |
| `tokens/colors.css` | Brand violet + blue, neutral ramp, status hues; light & dark semantic aliases; shadows. |
| `tokens/typography.css` | Geist / Geist Mono families, type scale, weights, semantic font roles. |
| `tokens/spacing.css` | 4px spacing scale, radii, borders, layout chrome, z-index, motion. |
| `tokens/fonts.css` | Geist + Geist Mono webfont import. |
| `assets/` | `frappe-logo.svg` (brand mark), `social-icons.svg` (import-source brand sprite). |
| `guidelines/*.html` | Foundation specimen cards (Colors, Type, Spacing) for the Design System tab. |
| `components/forms/` | `Button`, `IconButton`, `Input`, `Select`, `Checkbox`, `Switch`. |
| `components/display/` | `Card`, `Badge`, `Tag`, `Avatar` / `AvatarGroup`. |
| `components/navigation/` | `Tabs` (underline + pill). |
| `components/feedback/` | `Dialog`, `Toast` / `ToastViewport`, `Tooltip`. |
| `ui_kits/print-shop/` | Full-screen interactive recreation of the Frappe app (Dashboard, Orders, Production kanban, Job ticket, Invoicing). |
| `SKILL.md` | Agent-skill manifest so this system can be used as a Claude skill. |

Components are React (`.jsx`) with a sibling `.d.ts` (props contract) and `.prompt.md` (usage). They're compiled into a runtime bundle (`_ds_bundle.js`, auto-generated) and exposed on `window.FrappeDesignSystem_75694f`. Style components via the CSS custom properties above — never hard-code colors.

### Using the system
```html
<link rel="stylesheet" href="styles.css">
<!-- light by default; add data-theme="dark" on <html> for dark -->
<script src="_ds_bundle.js"></script>
<script>
  const { Button, Tabs, Badge } = window.FrappeDesignSystem_75694f;
</script>
```
