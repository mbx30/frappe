**Toast / ToastViewport** — transient confirmations ("Order saved", "Payment recorded"). Render one `ToastViewport`; push `Toast`es into it.

```jsx
<ToastViewport placement="bottom-right">
  <Toast tone="success" title="Invoice sent" message="INV-2048 emailed to Acme Co." onClose={dismiss} />
</ToastViewport>
```

Tones `neutral | success | warning | danger | info`. Auto-dismiss via `duration` (0 = sticky). Optional `action` slot.
