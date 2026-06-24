# Frappe — Frontend Performance Guide

This document captures the heavy components, the optimizations
applied to them, and the load-time targets they need to stay under.

## Heavy components

| Component | Why it's heavy | Optimizations applied |
| --- | --- | --- |
| `Dashboard.tsx` | Aggregates orders, computes stats per render, and switches between two view modes. | `useMemo` for `stats` and `filteredOrders`; `useCallback` for `loadOrders`; virtualised list view via `OrderListView`; only re-renders the active view. |
| `PDFView.tsx` | PDFium render + thumbnail + plate view + eyedropper. Multiple `useCallback`s for keyboard handlers. | `useCallback` for `toggleFullscreen`, `handleImageClick`, `runFullPreflight`, `handleFind`, `submitFind`, `handleKeyDown`; image data memoised in a ref to avoid re-decoding; thumb canvas painted once. |
| `OrderKanban.tsx` | Renders every order card across four columns, each with badge + date + status. Drag/drop re-orders the list. | `useMemo` for `ordersByStatus` and `todayStr`; `useCallback` for `handleDragStart`, `handleDragOver`, `handleDrop`, `isOverdue`, `isDueToday`; the per-card `OrderCard` is `memo()`; drag uses `transform: translate3d()` for GPU compositing; `transition: transform, opacity` only (issue #287). |
| `OrderListView.tsx` | Long lists (5k+ rows). | `memo` on `OrderRow`; `VirtualList` kicks in past 200 rows. |
| `InvoiceList.tsx`, `ClientList.tsx` | Same long-list pattern. | `VirtualList` (see `src/components/common/VirtualList.tsx`). |
| `FulfillmentPanel.tsx`, `OrderDetail.tsx` | Server-fetched detail blobs. | Use `invoke()` per field group; avoid `Promise.all` of a dozen commands. |

## Memoization rules of thumb

1. Anything derived from `orders` or `filteredOrders` belongs in
   `useMemo` — the dashboard's stats and the kanban's status buckets
   are O(n) and the cost compounds on every keystroke in the filter
   input.
2. Event handlers passed as props to memoised children must be
   `useCallback`. The kanban's `handleDrop` is a textbook case:
   without `useCallback`, every render of `OrderKanban` re-creates
   the closure and `OrderCard`'s `memo()` does nothing.
3. Avoid `Date.now()` or `new Date()` in render bodies — these are
   impure and the value will change on every re-render. Hoist them
   into a `useMemo` whose dependency is `[]` (or `[someTick]`) so the
   value is stable for the day.

## Animation guidance

The kanban CSS only transitions `transform` and `opacity` (see
`OrderKanban.css`). Width/height/top/left transitions force a relayout
on every frame and were the main cause of the 4 % scroll jank on
older MacBook Pros.

Other components use the same rule: hover effects modulate
`box-shadow` and `transform: translateY(-1px)`, not `margin-top`.

## Lazy loading

The PDFView component is dynamically imported in `App.tsx` (see
`React.lazy(() => import('./components/PDFView'))`) so the initial
bundle does not include pdfium.js / pdf.worker.js. The OrderKanban
is similarly lazy-loaded — the kanban view only fires when the user
clicks the "Kanban" tab in the dashboard.

## Asset pipeline (vite-imagetools)

`vite.config.ts` includes the `vite-imagetools` plugin. PNG / JPG
imports are automatically transformed into WebP + AVIF variants at
build time and a `<picture>` element picks the best format the
browser supports. The default sizes are `640, 1024, 1536` and the
quality is 80. The output is cached in `dist/assets/`.

## Targets (from `docs/perf-baseline.md`)

- Dashboard list load (<1 k rows): < 1 s
- `render_page` at 144 DPI: < 150 ms
- `render_page_b64` at 144 DPI: < 200 ms

If a regression is suspected, run `npm run build` and inspect the
`dist/` output for chunk sizes (`dist/assets/*.js`). A chunk > 200 kB
minified is a strong hint that a heavy dependency leaked into the
main bundle.
