/** Rust 側 frames::store::FrameStatus と同じ形（invoke('list_frames') の戻り値） */
export interface FrameStatus {
  id: string
  vendor: 'apple' | 'google'
  category: 'phone' | 'tablet' | 'laptop' | 'desktop' | 'display'
  /** 機種名のみ。向き・画面種別は orientation / display から UI が合成する */
  name: string
  orientation: 'portrait' | 'landscape'
  /** 折りたたみ端末の画面種別（iPhone Duo）。それ以外は null */
  display: 'inner' | 'outer' | null
  state: 'bundled' | 'imported' | 'missing'
  /** 取り込み済みの色スラッグ（例 "black-titanium"）。同梱は空 */
  variants: string[]
  source_url: string | null
}

/** capture_screenshots の devices 引数の要素。同梱デバイスは variant null */
export interface DeviceSelection {
  id: string
  variant: string | null
}

/** invoke('import_frames') の戻り値（Task 10 で使う） */
export interface ImportReport {
  imported: { id: string; variant: string }[]
  skipped: { file: string; reason: string }[]
}

/** Chrome に注入する手動操作オーバーレイの文言（Rust は翻訳表を持たない）。{label} {seconds} {frames} はテンプレートのまま渡す */
export interface OverlayLabels {
  startGif: string
  startPng: string
  startButton: string
  countdown: string
  cancel: string
  intro: string
  ready: string
  recording: string
  saving: string
  progress: string
}
