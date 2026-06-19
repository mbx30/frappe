**Select** — native dropdown styled to match `Input`. Pass `options` as strings or `{value,label}`.

```jsx
<Select label="Paper stock" options={['16pt Matte', '14pt Gloss', '100lb Uncoated']} />
<Select label="Status" placeholder="Choose…" options={[{value:'queued',label:'Queued'},{value:'press',label:'On press'}]} />
```

Sizes `sm | md | lg`. Same violet focus ring and red error border as Input.
