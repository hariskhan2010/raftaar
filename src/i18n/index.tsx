import { createContext, useContext } from 'react'
import type { TweakView } from '../api'
import { en, type Strings } from './en'
import { ur } from './ur'
import { hi } from './hi'
import { ar } from './ar'
import { es } from './es'
import { pt } from './pt'
import { tr } from './tr'
import { tweakText } from './tweaks'

export type { Strings }
export type Lang = 'en' | 'ur' | 'hi' | 'ar' | 'es' | 'pt' | 'tr'

export const LANGS: { code: Lang; native: string; rtl: boolean; locale: string }[] = [
  { code: 'en', native: 'English', rtl: false, locale: 'en' },
  { code: 'ur', native: 'اردو', rtl: true, locale: 'ur-PK' },
  { code: 'hi', native: 'हिन्दी', rtl: false, locale: 'hi-IN' },
  { code: 'ar', native: 'العربية', rtl: true, locale: 'ar' },
  { code: 'es', native: 'Español', rtl: false, locale: 'es' },
  { code: 'pt', native: 'Português (BR)', rtl: false, locale: 'pt-BR' },
  { code: 'tr', native: 'Türkçe', rtl: false, locale: 'tr' },
]

export const strings: Record<Lang, Strings> = { en, ur, hi, ar, es, pt, tr }

export const isRtl = (l: Lang) => LANGS.find((x) => x.code === l)?.rtl ?? false

/** First run: follow the Windows display language when we have it. */
export function detectLang(browserLangs: readonly string[]): Lang {
  for (const tag of browserLangs) {
    const base = tag.toLowerCase().split('-')[0]
    const hit = LANGS.find((l) => l.code === base)
    if (hit) return hit.code
  }
  return 'en'
}

export function tweakName(tw: TweakView, lang: Lang): string {
  if (lang === 'ur') return tw.name_ur
  return tweakText[lang]?.[tw.id]?.[0] ?? tw.name_en
}

export function tweakDesc(tw: TweakView, lang: Lang): string {
  if (lang === 'ur') return tw.desc_ur
  return tweakText[lang]?.[tw.id]?.[1] ?? tw.desc_en
}

export const LangContext = createContext<{ lang: Lang; t: Strings; setLang: (l: Lang) => void }>({
  lang: 'en',
  t: en,
  setLang: () => {},
})

export const useT = () => useContext(LangContext)

export function errText(e: unknown): string {
  return typeof e === 'string' ? e : e instanceof Error ? e.message : JSON.stringify(e)
}

export function formatDate(unix: number, lang: Lang): string {
  const locale = LANGS.find((l) => l.code === lang)?.locale
  return new Date(unix * 1000).toLocaleString(locale, { dateStyle: 'medium', timeStyle: 'short' })
}
