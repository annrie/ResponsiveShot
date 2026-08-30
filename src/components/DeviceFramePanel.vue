<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { openUrl, revealItemInDir } from '@tauri-apps/plugin-opener'
import type { DeviceSelection, FrameStatus, ImportReport } from '../types/frames'

defineProps<{ disabled: boolean }>()
const selected = defineModel<DeviceSelection[]>('selected', { required: true })
const shadow = defineModel<boolean>('shadow', { required: true })
const emit = defineEmits<{ status: [message: string] }>()

const frames = ref<FrameStatus[]>([])
const framesDir = ref('')
const vendorLabels: Record<FrameStatus['vendor'], string> = { apple: 'Apple', google: 'Google' }
const stateLabels: Record<FrameStatus['state'], string> = {
  bundled: '同梱',
  imported: '取り込み済み',
  missing: '未取り込み',
}

const groups = computed(() =>
  (['apple', 'google'] as const)
    .map(vendor => ({
      vendor,
      label: vendorLabels[vendor],
      items: frames.value.filter(f => f.vendor === vendor),
    }))
    .filter(g => g.items.length > 0)
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
    emit('status', `フレーム一覧の取得に失敗: ${e}`)
  }
  try {
    framesDir.value = await invoke<string>('get_frames_dir')
  } catch (e) {
    framesDir.value = ''
    emit('status', `取り込み先の取得に失敗: ${e}`)
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
    emit('status', `ブラウザを開けませんでした: ${e}`)
  }
}

const officialUrl = computed(
  () => frames.value.find(f => f.vendor === 'apple' && f.source_url)?.source_url ?? ''
)

const runImport = async (path: string) => {
  importing.value = true
  emit('status', 'フレームを取り込んでいます...')
  try {
    const report = await invoke<ImportReport>('import_frames', { path })
    await refresh()
    const names = report.imported.map(i => `${i.id} (${i.variant})`).join(', ')
    const skipped = report.skipped.length
      ? ` / スキップ ${report.skipped.length} 件: ${report.skipped.map(s => s.reason).join('; ')}`
      : ''
    emit(
      'status',
      report.imported.length
        ? `取り込み完了: ${names}${skipped}（保存先: ${framesDir.value}）`
        : `取り込めるフレームがありませんでした${skipped}`
    )
  } catch (e) {
    emit('status', `取り込みエラー: ${e}`)
  } finally {
    importing.value = false
  }
}

const openFramesDir = async () => {
  try {
    await revealItemInDir(framesDir.value)
  } catch (e) {
    emit('status', `Finder を開けませんでした: ${e}`)
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
      <h2 class="text-sm font-medium">デバイスフレーム</h2>
      <label class="flex items-center gap-2 cursor-pointer text-sm">
        <input type="checkbox" v-model="shadow" class="text-blue-500 rounded" />
        ドロップシャドウ
      </label>
    </div>
    <p class="text-xs text-gray-500 dark:text-gray-400 mb-4">
      選んだ端末の解像度で撮影し、フレームにはめ込んだ PNG を保存します（PNG 出力のみ。GIF では使えません）。
    </p>

    <div v-for="g in groups" :key="g.vendor" class="mb-4 last:mb-0">
      <h3 class="text-xs font-bold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-2">{{ g.label }}</h3>
      <div class="flex flex-wrap gap-3">
        <label
          v-for="f in g.items"
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
            {{ stateLabels[f.state] }}
          </span>
          <select
            v-if="f.state === 'imported' && f.variants.length > 1 && isSelected(f.id)"
            :value="variantOf(f.id)"
            @change="setVariant(f.id, ($event.target as HTMLSelectElement).value)"
            class="text-xs bg-gray-100 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded px-1 py-0.5"
          >
            <option v-for="v in f.variants" :key="v" :value="v">{{ v }}</option>
          </select>
          <button
            v-if="f.state === 'missing' && f.source_url"
            type="button"
            @click.prevent="openOfficial(f.source_url ?? '')"
            class="text-xs text-blue-500 hover:text-blue-600 dark:text-blue-400 underline"
          >
            公式
          </button>
        </label>
      </div>
      <div v-if="g.vendor === 'apple'" class="mt-3 flex flex-wrap items-center gap-2">
        <span class="text-xs text-gray-500 dark:text-gray-400 w-full basis-full">
          ① 「公式サイトを開く」で Apple Design Resources をブラウザで開き、② ブラウザで iPhone 16 の Product Bezels（DMG）をダウンロードしてください（進行状況はブラウザのダウンロード欄に表示されます。アプリはダウンロードしません）。③ ダウンロードした DMG ファイルを「DMG / PNG を取り込む」で選ぶと取り込まれます。
        </span>
        <button
          type="button"
          @click="openOfficial(officialUrl)"
          :disabled="!officialUrl"
          class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50"
        >
          公式サイトを開く
        </button>
        <button
          type="button"
          @click="importDmg"
          :disabled="importing"
          class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50"
        >
          DMG / PNG を取り込む
        </button>
        <button
          type="button"
          @click="importFolder"
          :disabled="importing"
          class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50"
        >
          フォルダを取り込む
        </button>
        <button
          type="button"
          @click="openFramesDir"
          :disabled="!framesDir"
          class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50"
        >
          取り込み先を Finder で開く
        </button>
      </div>
    </div>
    <p class="mt-4 text-xs text-gray-500 dark:text-gray-400">
      Apple のベゼルは Apple のライセンスに従いご自身の責任で使用してください。影の追加は Apple のガイドライン上は改変に当たります。Pixel のフレームは AOSP 由来（Apache 2.0）です。
    </p>
  </section>
</template>
