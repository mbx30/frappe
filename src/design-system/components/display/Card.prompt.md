**Card** — the default surface panel. Compose header, body, footer.

```jsx
<Card title="Order #1042" subtitle="Acme Co · due Fri" actions={<IconButton icon={<MoreHorizontal size={16}/>} label="More"/>}>
  …
</Card>
<Card interactive onClick={open}>Clickable summary</Card>
```

Props: `title`, `subtitle`, `actions`, `footer`, `interactive` (hover lift), `padding` (`none|sm|md|lg`). Radius `lg`, subtle shadow, 1px border.
