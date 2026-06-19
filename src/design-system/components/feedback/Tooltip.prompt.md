**Tooltip** — a small hover/focus label; wrap any trigger (especially `IconButton`) to explain it.

```jsx
<Tooltip label="Split quantity">
  <IconButton icon={<Scissors size={16} />} label="Split" />
</Tooltip>
<Tooltip label="Due Friday, 3pm" side="bottom">
  <Badge tone="warning">Due soon</Badge>
</Tooltip>
```

Props: `label`, `side` (`top|bottom|left|right`), `delay` (ms). Dark pill, mono-quiet caption text, fades in fast with no bounce. Appears on focus too (keyboard-accessible).
