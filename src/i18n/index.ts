import { createI18n } from 'vue-i18n'
import ja from '../locales/ja.json'
import en from '../locales/en.json'
import de from '../locales/de.json'
import es from '../locales/es.json'
import fr from '../locales/fr.json'
import ko from '../locales/ko.json'
import ptBR from '../locales/pt-BR.json'
import zhTW from '../locales/zh-TW.json'

export const SUPPORTED_LOCALES = ['ja', 'en', 'de', 'es', 'fr', 'ko', 'pt-BR', 'zh-TW'] as const
export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number]

export const LOCALE_NAMES: Record<SupportedLocale, string> = {
  ja: '日本語',
  en: 'English',
  de: 'Deutsch',
  es: 'Español',
  fr: 'Français',
  ko: '한국어',
  'pt-BR': 'Português (Brasil)',
  'zh-TW': '繁體中文',
}

const STORAGE_KEY = 'responsiveshot_language'

const isSupported = (v: string | null | undefined): v is SupportedLocale =>
  !!v && (SUPPORTED_LOCALES as readonly string[]).includes(v)

/** localStorage → navigator.language（先頭一致）→ 'en' */
export const detectLocale = (): SupportedLocale => {
  try {
    const saved = localStorage.getItem(STORAGE_KEY)
    if (isSupported(saved)) return saved
  } catch {
    /* localStorage が使えない環境では無視 */
  }
  const nav = (navigator.language || '').toLowerCase()
  if (nav.startsWith('ja')) return 'ja'
  if (nav.startsWith('zh')) return 'zh-TW'
  if (nav.startsWith('pt')) return 'pt-BR'
  if (nav.startsWith('ko')) return 'ko'
  if (nav.startsWith('fr')) return 'fr'
  if (nav.startsWith('es')) return 'es'
  if (nav.startsWith('de')) return 'de'
  return 'en'
}

export const i18n = createI18n({
  legacy: false,
  locale: detectLocale(),
  fallbackLocale: 'en',
  // overlay.* は Chrome 側の innerHTML 用に HTML を含む（Vue では描画しない）ため、intlify の HTML 警告を抑止
  warnHtmlMessage: false,
  messages: { ja, en, de, es, fr, ko, 'pt-BR': ptBR, 'zh-TW': zhTW },
})

export const setLocale = (locale: SupportedLocale) => {
  i18n.global.locale.value = locale
  try {
    localStorage.setItem(STORAGE_KEY, locale)
  } catch {
    /* ignore */
  }
  document.documentElement.lang = locale
}

document.documentElement.lang = i18n.global.locale.value
