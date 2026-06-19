**Dialog** — centered modal for confirmations and quick forms. Dismisses on scrim/Escape.

```jsx
<Dialog open={open} onClose={close} title="Delete workbook?"
  description="This removes all sheets. This can't be undone."
  footer={<><Button onClick={close}>Cancel</Button><Button variant="danger" onClick={confirm}>Delete</Button></>}>
  …optional body…
</Dialog>
```

Props: `open`, `onClose`, `title`, `description`, `footer`, `width`. Scrim blurs the canvas; panel pops in (no bounce).
