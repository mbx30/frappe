**Tabs** — the system's primary view-switcher. `underline` for top-level page nav, `pill` for segmented sub-filters. Supports icons and count pills.

```jsx
<Tabs variant="underline" tabs={[
  {value:'orders', label:'Orders', count:18},
  {value:'production', label:'Production', count:6},
  {value:'invoices', label:'Invoices'},
]} defaultValue="orders" onChange={setView} />

<Tabs variant="pill" tabs={['All','Open','Closed']} defaultValue="All" />
```

Controlled (`value`) or uncontrolled (`defaultValue`). Active underline is brand violet.
