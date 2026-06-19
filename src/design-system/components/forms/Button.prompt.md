**Button** — the primary action control; use `primary` for the single most important action on a view, `secondary` for everything else.

```jsx
<Button variant="primary" iconLeft={<Plus size={15} />}>New order</Button>
<Button variant="secondary">Cancel</Button>
<Button variant="danger" size="sm">Delete</Button>
```

Variants: `primary` (brand violet), `secondary` (bordered neutral), `subtle` (tinted violet), `ghost` (transparent), `danger`. Sizes `sm | md | lg`. Props: `loading`, `disabled`, `fullWidth`, `iconLeft`, `iconRight`. Hover lightens, press scales to 0.97 — never bounces.
