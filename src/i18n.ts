// i18n-ready string table — all user-facing text lives here.
// Locale-specific dictionaries live in `./<locale>.ts`.

import { en } from './i18n/en'

export type Locale = 'en'

const dictionaries: Record<Locale, Record<string, string>> = {
  en,
}

let currentLocale: Locale = 'en'

export function setLocale(locale: Locale) {
  currentLocale = locale
}

export function t(key: string, vars?: Record<string, string | number>): string {
  const template =
    dictionaries[currentLocale]?.[key] ?? dictionaries.en[key] ?? key
  if (!vars) return template
  return template.replace(/\{(\w+)\}/g, (_, name) =>
    vars[name]?.toString() ?? `{${name}}`,
  )
}
