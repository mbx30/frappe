**IconButton** — square icon-only button for toolbars, table rows, and dialog close affordances. Always pass `label` for accessibility.

```jsx
<IconButton icon={<MoreHorizontal size={16} />} label="More actions" />
<IconButton icon={<Trash2 size={16} />} label="Delete" variant="ghost" />
<IconButton icon={<Plus size={16} />} label="Add sheet" variant="subtle" />
```

Variants `primary | secondary | subtle | ghost`, sizes `sm | md | lg`. Press scales to 0.94.
