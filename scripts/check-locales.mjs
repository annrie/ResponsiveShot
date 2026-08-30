// 8 ロケールのキー集合と {placeholder} 集合が en.json と一致することを検証する
import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'

const dir = new URL('../src/locales/', import.meta.url).pathname
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

const files = readdirSync(dir).filter(f => f.endsWith('.json')).map(f => f.replace(/\.json$/, ''))
const missingFiles = REQUIRED.filter(l => !files.includes(l))
const en = flatten(JSON.parse(readFileSync(join(dir, 'en.json'), 'utf8')))
let errors = [...missingFiles.map(l => `missing locale file: ${l}.json`)]

for (const locale of files) {
  const m = flatten(JSON.parse(readFileSync(join(dir, `${locale}.json`), 'utf8')))
  for (const key of en.keys()) if (!m.has(key)) errors.push(`${locale}: missing key ${key}`)
  for (const [key, val] of m) {
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
