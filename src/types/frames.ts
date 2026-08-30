/** Rust 側 frames::store::FrameStatus と同じ形（invoke('list_frames') の戻り値） */
export interface FrameStatus {
  id: string
  vendor: 'apple' | 'google'
  category: 'phone' | 'tablet'
  name: string
  orientation: 'portrait' | 'landscape'
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
