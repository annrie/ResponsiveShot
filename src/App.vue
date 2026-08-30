<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'
import { useStorage, useColorMode } from '@vueuse/core'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useI18n } from 'vue-i18n'
import DeviceFramePanel from './components/DeviceFramePanel.vue'
import type { DeviceSelection, OverlayLabels } from './types/frames'
import { LOCALE_NAMES, SUPPORTED_LOCALES, setLocale, type SupportedLocale } from './i18n'

const { t, locale } = useI18n()
const changeLocale = (e: Event) => setLocale((e.target as HTMLSelectElement).value as SupportedLocale)

const colorPreference = useStorage('vueuse-color-scheme', 'auto')
useColorMode() // Activates UnoCSS dark listener

const url = useStorage('rs-url', 'https://example.com')
const urlHistory = useStorage<string[]>('rs-url-history', [])
const targetMode = useStorage<'viewport' | 'fullpage' | 'element'>('rs-mode', 'fullpage')
const outputFormat = useStorage<'png' | 'gif'>('rs-format', 'png')
const manualInteraction = useStorage('rs-manual-interaction', true)
const gifDuration = useStorage('rs-gif-duration', 3)
const startDelay = useStorage('rs-start-delay', 2)
const selectedSelector = useStorage('rs-selector', '')
const saveDir = useStorage('rs-save-dir', '')

const isCapturing = ref(false)
const isSelectingElement = ref(false)
const statusMessage = ref('')

const addToHistory = (newUrl: string) => {
  if (!newUrl) return
  const list = urlHistory.value.filter(u => u !== newUrl)
  list.unshift(newUrl)
  urlHistory.value = list.slice(0, 10)
}

const predefinedWidths = [320, 375, 640, 768, 1024, 1440, 1600]
const selectedWidths = useStorage<number[]>('rs-widths', predefinedWidths.slice())
const heightAuto = useStorage('rs-height-auto', true)
const selectedRatio = useStorage('rs-ratio', '16:9')
const customRatioW = useStorage('rs-custom-ratio-w', 5)
const customRatioH = useStorage('rs-custom-ratio-h', 4)
const selectedDevices = useStorage<DeviceSelection[]>('rs-devices', [])
const frameShadow = useStorage('rs-frame-shadow', false)
const frameBackground = useStorage('rs-frame-bg', 'transparent')

const ratioOptions = computed(() => [
  { value: '16:9', label: t('widths.ratio.r16x9.label'), description: t('widths.ratio.r16x9.description') },
  { value: '16:10', label: t('widths.ratio.r16x10.label'), description: t('widths.ratio.r16x10.description') },
  { value: '4:3', label: t('widths.ratio.r4x3.label'), description: t('widths.ratio.r4x3.description') },
  { value: '3:2', label: t('widths.ratio.r3x2.label'), description: t('widths.ratio.r3x2.description') },
  { value: '1:1', label: t('widths.ratio.r1x1.label'), description: t('widths.ratio.r1x1.description') },
  { value: '21:9', label: t('widths.ratio.r21x9.label'), description: t('widths.ratio.r21x9.description') },
  { value: '9:16', label: t('widths.ratio.r9x16.label'), description: t('widths.ratio.r9x16.description') },
  { value: 'custom', label: t('widths.ratio.custom.label'), description: t('widths.ratio.custom.description') },
])

const activeRatio = computed(() => {
  if (selectedRatio.value === 'custom') {
    const w = Number(customRatioW.value)
    const h = Number(customRatioH.value)
    return w > 0 && h > 0 ? w / h : 16 / 9
  }

  const [w, h] = selectedRatio.value.split(':').map(Number)
  return w > 0 && h > 0 ? w / h : 16 / 9
})

const viewportHeight = computed(() => {
  if (heightAuto.value || targetMode.value === 'fullpage') return null
  if (selectedWidths.value.length === 0) return null
  return Math.max(1, Math.round(Math.max(...selectedWidths.value) / activeRatio.value))
})

const ratioPreview = computed(() => {
  if (targetMode.value === 'fullpage') return t('widths.preview.fullpage')
  if (heightAuto.value) return t('widths.preview.auto')
  if (selectedWidths.value.length === 0) return t('widths.preview.selectWidth')
  return selectedWidths.value
    .slice()
    .sort((a, b) => a - b)
    .map(w => `${w}x${Math.max(1, Math.round(w / activeRatio.value))}`)
    .join(' / ')
})

onMounted(async () => {
  if (!saveDir.value) {
    try {
      saveDir.value = await invoke('get_default_save_dir')
    } catch (e) {
      console.error(e)
    }
  }
})

// Check/Uncheck all widths
const toggleAllWidths = () => {
  if (selectedWidths.value.length === predefinedWidths.length) {
    selectedWidths.value = []
  } else {
    selectedWidths.value = predefinedWidths.slice()
  }
}

// Select Save Directory Dialog
const selectSaveDir = async () => {
  const selected = await open({
    directory: true,
    multiple: false,
  })
  if (selected && !Array.isArray(selected)) {
    saveDir.value = selected
  }
}

// Trigger Element Selection
const selectElement = async () => {
  if (!url.value) {
    statusMessage.value = t('status.enterUrl')
    return
  }
  try {
    addToHistory(url.value)
    isSelectingElement.value = true
    statusMessage.value = t('status.selectingElement')
    const selector: string = await invoke('select_element', { url: url.value })
    if (selector) {
      selectedSelector.value = selector
      targetMode.value = 'element'
      statusMessage.value = t('status.elementSelected', { selector })
    } else {
      statusMessage.value = t('status.elementSelectCancelled')
    }
  } catch (err) {
    statusMessage.value = t('status.elementSelectError', { error: String(err) })
  } finally {
    isSelectingElement.value = false
  }
}

// Execute Capture
const captureScreenshots = async () => {
  if (!url.value) {
    statusMessage.value = t('status.enterUrl')
    return
  }
  if (!saveDir.value) {
    statusMessage.value = t('status.selectSaveDir')
    return
  }
  if (outputFormat.value === 'gif' && selectedWidths.value.length === 0) {
    statusMessage.value = t('status.selectWidthForGif')
    return
  }
  const devices = outputFormat.value === 'gif' ? [] : selectedDevices.value
  if (selectedWidths.value.length === 0 && devices.length === 0) {
    statusMessage.value = t('status.selectWidthOrDevice')
    return
  }
  const bg = frameBackground.value.trim()
  const bgIsTransparent = bg === '' || bg.toLowerCase() === 'transparent'
  if (devices.length > 0 && !bgIsTransparent && !/^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/.test(bg)) {
    statusMessage.value = t('status.invalidBackground')
    return
  }

  try {
    addToHistory(url.value)
    isCapturing.value = true
    statusMessage.value = t('status.starting')
    
    await invoke('capture_screenshots', {
      url: url.value,
      widths: selectedWidths.value,
      mode: targetMode.value,
      format: outputFormat.value,
      selector: selectedSelector.value,
      saveDir: saveDir.value,
      duration: outputFormat.value === 'gif' ? gifDuration.value : 0,
      delay: startDelay.value,
      manualInteraction: manualInteraction.value,
      viewportHeight: viewportHeight.value,
      devices,
      frameShadow: frameShadow.value,
      frameBackground: devices.length > 0 && !bgIsTransparent ? bg : null,
      overlay: {
        // vue-i18n は未指定の名前付きプレースホルダを空文字にするため、
        // Rust / JS 側でテンプレートのまま使うキーはプレースホルダ自身を値として渡す
        startGif: t('overlay.startGif'),
        startPng: t('overlay.startPng'),
        startButton: t('overlay.startButton', { label: '{label}', seconds: '{seconds}' }),
        countdown: t('overlay.countdown', { seconds: '{seconds}' }),
        cancel: t('overlay.cancel'),
        intro: t('overlay.intro'),
        ready: t('overlay.ready'),
        recording: t('overlay.recording', { seconds: '{seconds}' }),
        saving: t('overlay.saving'),
        progress: t('overlay.progress', { seconds: '{seconds}', frames: '{frames}' }),
      } satisfies OverlayLabels,
    })
    
    statusMessage.value = t('status.done')
  } catch (err) {
    statusMessage.value = t('status.captureError', { error: String(err) })
  } finally {
    isCapturing.value = false
  }
}

const abortCapture = async () => {
  try {
    await invoke('abort_capture')
    statusMessage.value = t('status.abortRequested')
  } catch (e) {
    console.error(e)
  }
}
</script>

<template>
  <main class="min-h-screen bg-gray-50 dark:bg-gray-900 text-gray-800 dark:text-gray-100 p-8 font-sans">
    <div class="max-w-3xl mx-auto space-y-6">
      <header class="text-center mb-8 relative">
        <div class="absolute right-0 top-0 flex gap-2">
          <select :value="locale" @change="changeLocale" :aria-label="t('language.label')" class="bg-gray-100 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 text-sm rounded-lg px-2 py-1 text-gray-700 dark:text-gray-300 focus:outline-none focus:ring-1 focus:ring-blue-500">
            <option v-for="l in SUPPORTED_LOCALES" :key="l" :value="l">{{ LOCALE_NAMES[l] }}</option>
          </select>
          <select v-model="colorPreference" class="bg-gray-100 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 text-sm rounded-lg px-2 py-1 text-gray-700 dark:text-gray-300 focus:outline-none focus:ring-1 focus:ring-blue-500">
            <option value="auto">{{ t('theme.system') }}</option>
            <option value="light">{{ t('theme.light') }}</option>
            <option value="dark">{{ t('theme.dark') }}</option>
          </select>
        </div>
        <h1 class="text-3xl font-bold bg-gradient-to-r from-blue-500 to-indigo-600 bg-clip-text text-transparent">
          ResponsiveShot
        </h1>
        <p class="text-sm text-gray-500 dark:text-gray-400 mt-2">{{ t('app.tagline') }}</p>
      </header>

      <!-- URL Input -->
      <section class="bg-white dark:bg-gray-800 p-6 rounded-xl shadow-sm border border-gray-100 dark:border-gray-700">
        <label class="block text-sm font-medium mb-2">{{ t('url.label') }}</label>
        <div class="flex flex-col gap-3">
          <input
            v-model="url"
            type="url"
            :placeholder="t('url.placeholder')"
            class="w-full px-4 py-3 text-lg border rounded-lg bg-gray-50 dark:bg-gray-900 border-gray-200 dark:border-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <div v-if="urlHistory.length > 0" class="flex items-center gap-2">
            <span class="text-xs text-gray-500 dark:text-gray-400">{{ t('url.history') }}</span>
            <select v-model="url" class="flex-1 text-sm bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 rounded-md px-2 py-1.5 focus:outline-none focus:ring-1 focus:ring-blue-500">
              <option v-for="h in urlHistory" :key="h" :value="h">{{ h }}</option>
            </select>
          </div>
        </div>
      </section>

      <!-- Capture Target Area -->
      <section class="bg-white dark:bg-gray-800 p-6 rounded-xl shadow-sm border border-gray-100 dark:border-gray-700">
        <h2 class="text-sm font-medium mb-4">{{ t('target.heading') }}</h2>
        <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">

          <label class="flex flex-col p-4 border rounded-lg cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
                 :class="{'border-blue-500 bg-blue-50 dark:bg-blue-900/20': targetMode === 'fullpage', 'border-gray-200 dark:border-gray-700': targetMode !== 'fullpage'}">
            <div class="flex items-center gap-2 mb-2">
              <input type="radio" value="fullpage" v-model="targetMode" class="text-blue-500" />
              <span class="font-medium">{{ t('target.fullpage') }}</span>
            </div>
            <p class="text-xs text-gray-500 dark:text-gray-400 pl-6">{{ t('target.fullpageHint') }}</p>
          </label>

          <label class="flex flex-col p-4 border rounded-lg cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
                 :class="{'border-blue-500 bg-blue-50 dark:bg-blue-900/20': targetMode === 'viewport', 'border-gray-200 dark:border-gray-700': targetMode !== 'viewport'}">
            <div class="flex items-center gap-2 mb-2">
              <input type="radio" value="viewport" v-model="targetMode" class="text-blue-500" />
              <span class="font-medium">{{ t('target.viewport') }}</span>
            </div>
            <p class="text-xs text-gray-500 dark:text-gray-400 pl-6">{{ t('target.viewportHint') }}</p>
          </label>

          <label class="flex flex-col p-4 border rounded-lg cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
                 :class="{'border-blue-500 bg-blue-50 dark:bg-blue-900/20': targetMode === 'element', 'border-gray-200 dark:border-gray-700': targetMode !== 'element'}">
            <div class="flex items-center gap-2 mb-2">
              <input type="radio" value="element" v-model="targetMode" class="text-blue-500" />
              <span class="font-medium">{{ t('target.element') }}</span>
            </div>
            <p class="text-xs text-gray-500 dark:text-gray-400 pl-6">{{ t('target.elementHint') }}</p>
          </label>

        </div>

        <h2 class="text-sm font-medium mt-6 mb-4">{{ t('output.heading') }}</h2>
        <div class="flex gap-4 flex-col sm:flex-row mb-4">
          <label class="flex items-center gap-3 p-4 border rounded-lg cursor-pointer flex-1 transition-colors"
                 :class="{'border-blue-500 bg-blue-50 dark:bg-blue-900/20': outputFormat === 'png', 'border-gray-200 dark:border-gray-700': outputFormat !== 'png'}">
            <input type="radio" value="png" v-model="outputFormat" class="text-blue-500 w-5 h-5" />
            <div class="flex flex-col">
              <span class="font-bold flex items-center gap-2"><div class="i-carbon-image text-lg"></div> {{ t('output.png') }}</span>
              <span class="text-xs text-gray-500 dark:text-gray-400 mt-1">{{ t('output.pngHint') }}</span>
            </div>
          </label>
          <label class="flex items-center gap-3 p-4 border rounded-lg cursor-pointer flex-1 transition-colors"
                 :class="{'border-blue-500 bg-blue-50 dark:bg-blue-900/20': outputFormat === 'gif', 'border-gray-200 dark:border-gray-700': outputFormat !== 'gif'}">
            <input type="radio" value="gif" v-model="outputFormat" class="text-blue-500 w-5 h-5" />
            <div class="flex flex-col">
              <span class="font-bold flex items-center gap-2"><div class="i-carbon-video text-lg"></div> {{ t('output.gif') }}</span>
              <span class="text-xs text-gray-500 dark:text-gray-400 mt-1">{{ t('output.gifHint') }}</span>
            </div>
          </label>
        </div>

        <label class="flex items-center gap-3 p-4 border rounded-lg cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
               :class="{'border-blue-500 bg-blue-50 dark:bg-blue-900/20': manualInteraction, 'border-gray-200 dark:border-gray-700': !manualInteraction}">
          <input type="checkbox" v-model="manualInteraction" class="text-blue-500 w-5 h-5 rounded" />
          <div class="flex flex-col">
            <span class="font-bold text-gray-800 dark:text-gray-200">{{ t('output.manualInteraction') }}</span>
            <span class="text-xs text-gray-500 dark:text-gray-400 mt-1">{{ t('output.manualInteractionHint') }}</span>
          </div>
        </label>

        <!-- Delay setting (common to PNG and GIF) -->
        <div class="mt-4 p-5 bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg flex flex-col gap-5">
          <div class="flex flex-col sm:flex-row items-center justify-between gap-4">
            <div class="text-sm">
              <span class="font-bold text-gray-800 dark:text-gray-200">{{ t('output.startDelay') }}</span>
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">{{ t('output.startDelayHint') }}</p>
            </div>
            <div class="flex items-center gap-3 bg-white dark:bg-gray-800 p-2 rounded border border-gray-200 dark:border-gray-700">
              <input type="range" v-model.number="startDelay" min="1" max="10" step="1" class="w-24 accent-blue-500" />
              <span class="font-mono text-blue-500 font-bold w-12 text-center">{{ t('output.startDelayUnit', { seconds: startDelay }) }}</span>
            </div>
          </div>
        </div>

        <!-- Extra settings for GIF Mode -->
        <div v-show="outputFormat === 'gif'" class="mt-4 p-5 bg-gray-50 dark:bg-gray-900 border border-blue-100 dark:border-blue-900/30 rounded-lg flex flex-col gap-5">
          <div class="flex flex-col sm:flex-row items-center justify-between gap-4">
            <div class="text-sm">
              <span class="font-bold text-gray-800 dark:text-gray-200">{{ t('output.gifDuration') }}</span>
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">{{ t('output.gifDurationHint') }}</p>
            </div>
            <div class="flex items-center gap-3 bg-white dark:bg-gray-800 p-2 rounded border border-gray-200 dark:border-gray-700">
              <input type="range" v-model.number="gifDuration" min="1" max="20" step="1" class="w-24 accent-blue-500" />
              <span class="font-mono text-blue-500 font-bold w-12 text-center">{{ t('output.gifDurationUnit', { seconds: gifDuration }) }}</span>
            </div>
          </div>
        </div>

        <div v-show="targetMode === 'element'" class="mt-4 p-4 bg-gray-50 dark:bg-gray-900 rounded-lg flex items-center justify-between">
          <div class="text-sm truncate mr-4">
            {{ t('target.selectorLabel') }}<span class="font-mono text-blue-500">{{ selectedSelector || t('target.selectorEmpty') }}</span>
          </div>
          <button
            @click="selectElement"
            class="px-4 py-2 bg-indigo-100 hover:bg-indigo-200 dark:bg-indigo-900 dark:hover:bg-indigo-800 text-indigo-700 dark:text-indigo-200 text-sm font-medium rounded-lg transition-colors whitespace-nowrap"
            :disabled="isSelectingElement"
          >
            {{ t('target.selectElement') }}
          </button>
        </div>
      </section>

      <!-- Width Selection -->
      <section class="bg-white dark:bg-gray-800 p-6 rounded-xl shadow-sm border border-gray-100 dark:border-gray-700">
        <div class="flex justify-between items-center mb-4">
          <h2 class="text-sm font-medium">{{ t('widths.heading') }}</h2>
          <button @click="toggleAllWidths" class="text-xs text-blue-500 hover:text-blue-600 dark:text-blue-400">
            {{ t('widths.toggleAll') }}
          </button>
        </div>
        <div class="flex flex-wrap gap-3">
          <label v-for="w in predefinedWidths" :key="w" 
                 class="flex items-center gap-2 px-3 py-2 border border-gray-200 dark:border-gray-700 rounded-md cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
                 :class="{'border-blue-400 bg-blue-50 dark:bg-blue-900/20': selectedWidths.includes(w)}">
            <input type="checkbox" :value="w" v-model="selectedWidths" class="text-blue-500 rounded" />
            <span class="text-sm font-mono">{{ w }}px</span>
          </label>
        </div>

        <div class="mt-5 border-t border-gray-100 dark:border-gray-700 pt-5">
          <label class="flex items-start gap-3 cursor-pointer">
            <input type="checkbox" v-model="heightAuto" class="mt-1 text-blue-500 rounded" />
            <div class="flex flex-col">
              <span class="text-sm font-bold text-gray-800 dark:text-gray-200">{{ t('widths.heightAuto') }}</span>
              <span class="text-xs text-gray-500 dark:text-gray-400 mt-1">{{ t('widths.heightAutoHint') }}</span>
            </div>
          </label>

          <div class="mt-4" :class="{'opacity-45 pointer-events-none': heightAuto || targetMode === 'fullpage'}">
            <div class="flex items-center justify-between gap-3 mb-3">
              <h3 class="text-xs font-bold text-gray-500 dark:text-gray-400 uppercase tracking-wide">{{ t('widths.ratioHeading') }}</h3>
              <span class="text-xs font-mono text-blue-600 dark:text-blue-400 truncate">{{ ratioPreview }}</span>
            </div>

            <div class="grid grid-cols-2 sm:grid-cols-4 gap-2">
              <label
                v-for="ratio in ratioOptions"
                :key="ratio.value"
                class="min-h-16 flex items-center gap-2 p-3 border rounded-lg cursor-pointer transition-colors bg-gray-50 dark:bg-gray-900 hover:bg-white dark:hover:bg-gray-800"
                :class="{'border-blue-500 bg-blue-50 dark:bg-blue-900/20': selectedRatio === ratio.value, 'border-gray-200 dark:border-gray-700': selectedRatio !== ratio.value}"
              >
                <input type="radio" :value="ratio.value" v-model="selectedRatio" class="text-blue-500" />
                <span class="flex flex-col min-w-0">
                  <span class="text-sm font-mono font-bold">{{ ratio.label }}</span>
                  <span class="text-xs text-gray-500 dark:text-gray-400 truncate">{{ ratio.description }}</span>
                </span>
              </label>
            </div>

            <div v-show="selectedRatio === 'custom'" class="mt-3 flex flex-wrap items-center gap-2 p-3 bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg">
              <span class="text-xs font-medium text-gray-500 dark:text-gray-400">{{ t('widths.customRatio') }}</span>
              <input
                v-model.number="customRatioW"
                type="number"
                min="1"
                class="w-20 px-3 py-2 text-sm font-mono border rounded-md bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700 focus:outline-none focus:ring-1 focus:ring-blue-500"
              />
              <span class="text-gray-400">:</span>
              <input
                v-model.number="customRatioH"
                type="number"
                min="1"
                class="w-20 px-3 py-2 text-sm font-mono border rounded-md bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700 focus:outline-none focus:ring-1 focus:ring-blue-500"
              />
            </div>
          </div>
        </div>
      </section>

      <!-- Device Frames -->
      <DeviceFramePanel
        v-model:selected="selectedDevices"
        v-model:shadow="frameShadow"
        v-model:background="frameBackground"
        :disabled="outputFormat === 'gif'"
        @status="statusMessage = $event"
      />

      <!-- Save Directory -->
      <section class="bg-white dark:bg-gray-800 p-6 rounded-xl shadow-sm border border-gray-100 dark:border-gray-700">
        <label class="block text-sm font-medium mb-2">{{ t('saveDir.label') }}</label>
        <div class="flex gap-2">
          <input
            v-model="saveDir"
            type="text"
            readonly
            :placeholder="t('saveDir.placeholder')"
            class="flex-1 px-4 py-2 border rounded-lg bg-gray-50 dark:bg-gray-900 border-gray-200 dark:border-gray-700 text-sm focus:outline-none cursor-not-allowed text-gray-500"
          />
          <button
            @click="selectSaveDir"
            class="px-4 py-2 bg-gray-200 hover:bg-gray-300 dark:bg-gray-800 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-100 font-medium rounded-lg transition-colors text-sm dark:shadow-none border border-transparent dark:border-gray-700"
          >
            {{ t('saveDir.browse') }}
          </button>
        </div>
      </section>

      <!-- Action & Status -->
      <section class="pt-4 flex flex-col items-center">
        <div class="flex gap-4 w-full sm:w-auto">
          <button 
            v-if="!isCapturing"
            @click="captureScreenshots"
            :disabled="isSelectingElement"
            class="flex-1 w-full sm:w-auto px-12 py-4 bg-blue-600 hover:bg-blue-700 focus:ring-4 focus:ring-blue-300 dark:focus:ring-blue-800 text-white font-bold text-lg rounded-xl shadow-lg dark:shadow-none transition-all flex items-center justify-center gap-3 disabled:opacity-50"
          >
            <span class="i-carbon-camera text-2xl"></span>
            {{ t('actions.capture') }}
          </button>

          <button
            v-if="isCapturing"
            @click="abortCapture"
            class="flex-1 w-full sm:w-auto px-12 py-4 bg-red-600 hover:bg-red-700 focus:ring-4 focus:ring-red-300 dark:focus:ring-red-800 text-white font-bold text-lg rounded-xl shadow-lg dark:shadow-none transition-all flex items-center justify-center gap-3 animate-pulse"
          >
            <span class="i-carbon-stop-outline text-2xl"></span>
            {{ t('actions.abort') }}
          </button>
        </div>
        <p v-if="statusMessage" class="mt-4 text-sm font-medium text-center" :class="{'text-blue-500': isCapturing || isSelectingElement, 'text-gray-600 dark:text-gray-400': !isCapturing && !isSelectingElement}">
          <span v-if="isCapturing" class="i-carbon-circle-dash animate-spin inline-block align-middle mr-2 text-lg"></span>
          <span class="align-middle">{{ statusMessage }}</span>
        </p>
      </section>

    </div>
  </main>
</template>
