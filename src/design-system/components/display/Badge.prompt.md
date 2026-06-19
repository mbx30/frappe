**Badge** — pill for job status, counts and labels. Use `dot` for live status.

```jsx
<Badge tone="success" dot>Shipped</Badge>
<Badge tone="warning" dot>Awaiting art</Badge>
<Badge tone="info">Queued</Badge>
<Badge tone="brand">Rush</Badge>
```

Tones: `neutral | brand | success | warning | danger | info`. Sizes `sm | md`. Print-shop mapping: queued→info, on-press→brand, shipped→success, overdue→danger, awaiting-art→warning.
