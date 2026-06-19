**Avatar / AvatarGroup** — user/operator identity. Initials fallback is auto-colored from the name (deterministic).

```jsx
<Avatar name="Dana Ruiz" />
<Avatar name="Max Bowen" src="/people/max.jpg" size="lg" />
<AvatarGroup names={["Dana Ruiz","Max Bowen","Priya Shah","Lee Ortiz","Sam Kade"]} max={3} />
```

Sizes `xs | sm | md | lg`. AvatarGroup overlaps and shows `+N` past `max`.
