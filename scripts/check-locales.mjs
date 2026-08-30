// 8 ロケールのキー集合と {placeholder} 集合が en.json と一致することを検証する
import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const dir = fileURLToPath(new URL('../src/locales/', import.meta.url))
const REQUIRED = ['ja', 'en', 'de', 'es', 'fr', 'ko', 'pt-BR', 'zh-TW']

const flatten = (obj, prefix = '', out = new Map()) => {
  for (const [k, v] of Object.entries(obj)) {
    const key = prefix ? `${prefix}.${k}` : k
    if (v && typeof v === 'object') flatten(v, key, out)
    else out.set(key, String(v))
  }
  return out
}
const placeholders = s => new Set([...s.matchAll(/\{(\w+)\}/g)].map(m => m[1]))
// vue-i18n の特殊文字チェック用: @ はリンクメッセージ、| は複数形の区切り、
// {name} 以外の { } は補間構文の誤り検出に使う。正しい {name} を除去した残りに
// @ | { } のいずれかが残っていれば、生テキストに紛れ込んだ構文エラーとみなす
const PLACEHOLDER_RE = /\{[A-Za-z][A-Za-z0-9_]*\}/g
const hasSpecialChar = s => /[@|{}]/.test(s.replace(PLACEHOLDER_RE, ''))

const files = readdirSync(dir).filter(f => f.endsWith('.json')).map(f => f.replace(/\.json$/, ''))
const missingFiles = REQUIRED.filter(l => !files.includes(l))
let errors = [...missingFiles.map(l => `missing locale file: ${l}.json`)]

let en
try {
  en = flatten(JSON.parse(readFileSync(join(dir, 'en.json'), 'utf8')))
} catch (e) {
  errors.push(`en.json: invalid JSON: ${e.message}`)
  console.error(errors.join('\n'))
  process.exit(1)
}

for (const [key, val] of en) {
  if (hasSpecialChar(val)) errors.push(`en.json: ${key}: contains vue-i18n special character`)
}

for (const locale of files) {
  const file = `${locale}.json`
  let m
  try {
    m = flatten(JSON.parse(readFileSync(join(dir, file), 'utf8')))
  } catch (e) {
    errors.push(`${file}: invalid JSON: ${e.message}`)
    continue
  }
  for (const key of en.keys()) if (!m.has(key)) errors.push(`${locale}: missing key ${key}`)
  for (const [key, val] of m) {
    if (hasSpecialChar(val)) errors.push(`${file}: ${key}: contains vue-i18n special character`)
    if (!en.has(key)) errors.push(`${locale}: extra key ${key}`)
    else {
      if (val.trim() === '') errors.push(`${locale}: empty value ${key}`)
      const a = [...placeholders(en.get(key))].sort().join(','), b = [...placeholders(val)].sort().join(',')
      if (a !== b) errors.push(`${locale}: placeholders differ for ${key} (en: ${a || '-'} / ${locale}: ${b || '-'})`)
    }
  }
}
if (errors.length) { console.error(errors.join('\n')); process.exit(1) }
console.log(`check-locales: ${files.length} locales, ${en.size} keys OK`)
