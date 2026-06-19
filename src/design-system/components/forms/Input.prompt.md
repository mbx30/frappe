**Input** — labelled text field with focus ring, error state, leading icon and suffix. Use `mono` for money, quantities, and IDs so figures align.

```jsx
<Input label="Job name" placeholder="500 matte business cards" />
<Input label="Unit price" mono iconLeft={<DollarSign size={14} />} suffix="ea" />
<Input label="Email" error="That address is already in use." />
```

Sizes `sm | md | lg`. Border goes violet on focus (3px ring), red on `error`.
