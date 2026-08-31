<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { openUrl } from '@tauri-apps/plugin-opener'
import type { DeviceSelection, FrameStatus, ImportReport } from '../types/frames'
import type { StatusMessage } from '../types/status'

const { t } = useI18n()

defineProps<{ disabled: boolean }>()
const selected = defineModel<DeviceSelection[]>('selected', { required: true })
const shadow = defineModel<boolean>('shadow', { required: true })
const emulateMobile = defineModel<boolean>('emulateMobile', { required: true })

/** 'transparent' か '#rrggbb'（'#rgb' も可）。App.vue 側で rs-frame-bg に永続化 */
const background = defineModel<string>('background', { required: true })

const HEX_RE = /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/
const isValidHex = (s: string) => HEX_RE.test(s.trim())

type BgMode = 'transparent' | 'white' | 'black' | 'custom'
const bgMode = computed<BgMode>(() => {
  const v = background.value.trim().toLowerCase()
  if (v === 'transparent') return 'transparent'
  if (v === '#ffffff' || v === '#fff') return 'white'
  if (v === '#000000' || v === '#000') return 'black'
  return 'custom'
})
const setBgMode = (mode: BgMode) => {
  if (mode === 'transparent') background.value = 'transparent'
  else if (mode === 'white') background.value = '#ffffff'
  else if (mode === 'black') background.value = '#000000'
  else if (bgMode.value !== 'custom') background.value = '#f5f5f5'
}
const backgroundInvalid = computed(() => bgMode.value === 'custom' && !isValidHex(background.value))

const emit = defineEmits<{ status: [message: StatusMessage] }>()

const frames = ref<FrameStatus[]>([])
const framesDir = ref('')
const vendorLabel = (v: FrameStatus['vendor']) => t(`frames.vendor.${v}`)
const categoryOrder: FrameStatus['category'][] = ['phone', 'tablet', 'laptop', 'desktop', 'display']
const categoryLabel = (c: FrameStatus['category']) => t(`frames.category.${c}`)
const APPLE_DESIGN_RESOURCES = 'https://developer.apple.com/design/resources/#product-bezels'
const stateLabel = (s: FrameStatus['state']) => t(`frames.state.${s}`)

const groups = computed(() =>
  (['apple', 'google'] as const)
    .map(vendor => ({
      vendor,
      label: vendorLabel(vendor),
      sections: categoryOrder
        .map(category => ({
          category,
          label: categoryLabel(category),
          items: frames.value.filter(f => f.vendor === vendor && f.category === category),
        }))
        .filter(s => s.items.length > 0),
    }))
    .filter(g => g.sections.length > 0)
)

/** list_frames の結果に合わせて選択を整える: 無い/未取り込みは外し、無い色は先頭の色にする */
const reconcile = () => {
  selected.value = selected.value.flatMap((sel): DeviceSelection[] => {
    const f = frames.value.find(x => x.id === sel.id)
    if (!f || f.state === 'missing') return []
    if (f.state === 'bundled') return [{ id: sel.id, variant: null }]
    const variant = sel.variant && f.variants.includes(sel.variant) ? sel.variant : f.variants[0]
    return [{ id: sel.id, variant }]
  })
}

const refresh = async () => {
  try {
    frames.value = await invoke<FrameStatus[]>('list_frames')
    reconcile()
  } catch (e) {
    emit('status', { key: 'frames.status.listFailed', params: { error: e } })
  }
  try {
    framesDir.value = await invoke<string>('get_frames_dir')
  } catch (e) {
    framesDir.value = ''
    emit('status', { key: 'frames.status.dirFailed', params: { error: e } })
  }
}

const isSelected = (id: string) => selected.value.some(s => s.id === id)

const toggle = (f: FrameStatus) => {
  if (f.state === 'missing') return
  if (isSelected(f.id)) {
    selected.value = selected.value.filter(s => s.id !== f.id)
  } else {
    selected.value = [
      ...selected.value,
      { id: f.id, variant: f.state === 'imported' ? f.variants[0] : null },
    ]
  }
}

onMounted(refresh)

const importing = ref(false)

const openOfficial = async (url: string) => {
  try {
    await openUrl(url)
  } catch (e) {
    emit('status', { key: 'frames.status.openFailed', params: { error: e } })
  }
}

const runImport = async (path: string) => {
  importing.value = true
  emit('status', { key: 'frames.status.importing' })
  try {
    const report = await invoke<ImportReport>('import_frames', { path })
    await refresh()
    const names = report.imported.map(i => `${i.id} (${i.variant})`).join(', ')
    const skipped = report.skipped.length
      ? {
          key: 'frames.status.skipped',
          params: { count: report.skipped.length, reasons: report.skipped.map(s => s.reason).join('; ') },
        }
      : ''
    emit(
      'status',
      report.imported.length
        ? { key: 'frames.status.imported', params: { names, skipped, dir: framesDir.value } }
        : { key: 'frames.status.nothingImported', params: { skipped } }
    )
  } catch (e) {
    emit('status', { key: 'frames.status.importError', params: { error: e } })
  } finally {
    importing.value = false
  }
}

const importDmg = async () => {
  const picked = await open({ multiple: false, directory: false })
  if (typeof picked === 'string') await runImport(picked)
}

const importFolder = async () => {
  const picked = await open({ multiple: false, directory: true })
  if (typeof picked === 'string') await runImport(picked)
}

const variantOf = (id: string) => selected.value.find(s => s.id === id)?.variant ?? null

const setVariant = (id: string, variant: string) => {
  selected.value = selected.value.map(s => (s.id === id ? { id, variant } : s))
}

defineExpose({ refresh })
</script>

<template>
  <section
    class="bg-white dark:bg-gray-800 p-6 rounded-xl shadow-sm border border-gray-100 dark:border-gray-700"
    :class="{ 'opacity-45 pointer-events-none': disabled }"
  >
    <div class="flex justify-between items-center mb-1">
      <h2 class="text-sm font-medium">{{ t('frames.heading') }}</h2>
      <div class="flex items-center gap-4 text-sm">
        <label class="flex items-center gap-1">
          <span class="text-xs text-gray-500 dark:text-gray-400">{{ t('frames.background.label') }}</span>
          <select
            :value="bgMode"
            @change="setBgMode(($event.target as HTMLSelectElement).value as BgMode)"
            class="text-xs text-gray-800 dark:text-gray-100 bg-gray-100 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded px-1 py-0.5"
          >
            <option value="transparent">{{ t('frames.background.transparent') }}</option>
            <option value="white">{{ t('frames.background.white') }}</option>
            <option value="black">{{ t('frames.background.black') }}</option>
            <option value="custom">{{ t('frames.background.custom') }}</option>
          </select>
          <input
            v-if="bgMode === 'custom'"
            :value="background"
            @input="background = ($event.target as HTMLInputElement).value"
            type="text"
            :placeholder="t('frames.background.customPlaceholder')"
            spellcheck="false"
            class="w-24 text-xs font-mono text-gray-800 dark:text-gray-100 bg-gray-100 dark:bg-gray-900 border rounded px-1 py-0.5"
            :class="backgroundInvalid ? 'border-red-400' : 'border-gray-200 dark:border-gray-700'"
          />
        </label>
        <label class="flex items-center gap-2 cursor-pointer">
          <input type="checkbox" v-model="shadow" class="text-blue-500 rounded" />
          {{ t('frames.shadow') }}
        </label>
        <label class="flex items-center gap-2 cursor-pointer">
          <input type="checkbox" v-model="emulateMobile" class="text-blue-500 rounded" />
          {{ t('frames.emulateMobile') }}
        </label>
      </div>
    </div>
    <p class="text-xs text-gray-500 dark:text-gray-400 mb-4">
      {{ t('frames.intro') }}
    </p>

    <div v-for="g in groups" :key="g.vendor" class="mb-4 last:mb-0">
      <h3 class="text-xs font-bold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-2">{{ g.label }}</h3>
      <div v-for="s in g.sections" :key="s.category" class="mb-3 last:mb-0">
        <h4 class="text-xs text-gray-500 dark:text-gray-400 mb-2">{{ s.label }}</h4>
        <div class="flex flex-wrap gap-3">
          <label
            v-for="f in s.items"
            :key="f.id"
            class="flex items-center gap-2 px-3 py-2 border border-gray-200 dark:border-gray-700 rounded-md transition-colors"
            :class="[
              f.state === 'missing'
                ? 'opacity-60 cursor-not-allowed'
                : 'cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-700',
              { 'border-blue-400 bg-blue-50 dark:bg-blue-900/20': isSelected(f.id) },
            ]"
          >
            <input
              type="checkbox"
              :checked="isSelected(f.id)"
              :disabled="f.state === 'missing'"
              @change="toggle(f)"
              class="text-blue-500 rounded"
            />
            <span class="text-sm">{{ f.name }}</span>
            <span
              class="text-xs px-1.5 py-0.5 rounded"
              :class="
                f.state === 'missing'
                  ? 'bg-gray-200 dark:bg-gray-700 text-gray-600 dark:text-gray-300'
                  : 'bg-green-100 dark:bg-green-900/40 text-green-700 dark:text-green-300'
              "
            >
              {{ stateLabel(f.state) }}
            </span>
            <select
              v-if="f.state === 'imported' && f.variants.length > 1 && isSelected(f.id)"
              :value="variantOf(f.id)"
              @change="setVariant(f.id, ($event.target as HTMLSelectElement).value)"
              class="text-xs text-gray-800 dark:text-gray-100 bg-gray-100 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded px-1 py-0.5"
            >
              <option v-for="v in f.variants" :key="v" :value="v">{{ v }}</option>
            </select>
            <button
              v-if="f.state === 'missing' && f.source_url"
              type="button"
              @click.prevent="openOfficial(f.source_url ?? '')"
              class="text-xs text-blue-500 hover:text-blue-600 dark:text-blue-400 underline"
            >
              {{ t('frames.official') }}
            </button>
          </label>
        </div>
      </div>
      <div v-if="g.vendor === 'apple'" class="mt-3 flex flex-wrap items-center gap-2">
        <span class="text-xs text-gray-500 dark:text-gray-400 w-full basis-full">
          {{ t('frames.hint') }}
        </span>
        <button
          type="button"
          @click="openOfficial(APPLE_DESIGN_RESOURCES)"
          class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-100 border-0"
        >
          {{ t('frames.openOfficial') }}
        </button>
        <button
          type="button"
          @click="importDmg"
          :disabled="importing"
          class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-100 border-0 disabled:opacity-50"
        >
          {{ t('frames.importDmg') }}
        </button>
        <button
          type="button"
          @click="importFolder"
          :disabled="importing"
          class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-100 border-0 disabled:opacity-50"
        >
          {{ t('frames.importFolder') }}
        </button>
      </div>
    </div>
    <p class="mt-4 text-xs text-gray-500 dark:text-gray-400">
      {{ t('frames.license') }}
    </p>
  </section>
</template>
