/** ステータス表示の元データ。表示文ではなくキーとパラメータを保持し、言語切替時に再翻訳する */
export interface StatusMessage {
  key: string
  /** 値に StatusMessage を入れると、表示時にそれも翻訳される（取り込み結果のスキップ文など） */
  params?: Record<string, unknown>
}

export const isStatusMessage = (v: unknown): v is StatusMessage =>
  typeof v === 'object' && v !== null && typeof (v as StatusMessage).key === 'string'
