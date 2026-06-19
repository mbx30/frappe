---
name: frappe-design
description: Use this skill to generate well-branded interfaces and assets for Frappe, the local-first print-shop management app (print MIS — quoting, orders, prepress, production, invoicing). Use for production code or throwaway prototypes/mocks. Contains essential design guidelines, colors, type, fonts, assets, and a UI kit of components and full screens for prototyping.
user-invocable: true
---

# Frappe Design System

Frappe is a fast, local-first desktop app for **print shops** — a print MIS spanning estimating & quoting, order management, prepress & art approval, production (kanban + job tickets), shipping & pickup, invoicing, deposits & POS payments, inventory, and QuickBooks sync. The aesthetic is **utilitarian, dense, and clear**: a confident violet brand over neutral grays, first-class light AND dark themes, tab-driven navigation, and mono tabular figures for all money/quantities/IDs.

## How to use this skill

1. **Read `readme.md` first** — it is the full design guide: product context, sources, content fundamentals (voice/tone), visual foundations (color, type, space, motion, interaction), and iconography. Then explore the files referenced below.
2. **Foundations** live in `styles.css` → `tokens/*.css`. Link `styles.css` and build with the **semantic** CSS custom properties (`--surface-card`, `--text-primary`, `--border-default`, `--brand`, status triples) so light/dark both work. Add `data-theme="dark"` on a root element for dark.
3. **Components** are in `components/<group>/` (`forms`, `display`, `navigation`, `feedback`) as React `.jsx` with a `.d.ts` contract and `.prompt.md` usage notes. Read the `.prompt.md` for each before using it.
4. **The UI kit** is `ui_kits/print-shop/` — an interactive recreation of the Frappe app (sign-in/onboarding, dashboard, orders, production kanban, job ticket, invoicing). Lift screens and patterns from here.
5. **Assets** are in `assets/` (`frappe-logo.svg`, `social-icons.svg`). Icons are **Lucide** (lucide.dev) — outline, 2px stroke. No emoji, no hand-drawn SVG icons.

## Working modes

- **Visual artifacts** (slides, mocks, throwaway prototypes): **copy assets out** of this folder into your output location and produce **static/standalone HTML** the user can open. Reference `styles.css` and the Lucide CDN; mirror the component styles (or load `_ds_bundle.js` if present) rather than hand-rolling new visual languages.
- **Production code**: copy assets, read the rules here, reference the token CSS, and reimplement components against the `.d.ts` contracts. Install `lucide-react` for icons.

## If invoked with no specific request

Ask what they want to build or design, ask a few focused questions (surface, audience, light/dark, scope, variations), then act as an expert Frappe designer who outputs HTML artifacts **or** production code depending on the need. Match Frappe's voice (sentence case, plain operational language, print-shop vocabulary) and visual rules (violet accent used sparingly, neutral surfaces, mono tabular figures, quick no-bounce motion).

## Reference

- Source product: <https://github.com/mbx30/frappe> — explore the repo and its Issues (#1–#18) for the product roadmap before building new surfaces.
