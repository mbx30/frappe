**Checkbox** — labelled checkbox; supports `indeterminate` for "select all" headers.

```jsx
<Checkbox label="Rush job (+25%)" defaultChecked />
<Checkbox label="Select all" indeterminate onChange={...} />
<Checkbox label="Email proof to customer" hint="Sent when art is approved" />
```

Controlled with `checked`/`onChange` or uncontrolled with `defaultChecked`.
