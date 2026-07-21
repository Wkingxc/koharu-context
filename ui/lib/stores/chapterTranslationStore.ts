'use client'

import { create } from 'zustand'

import type { LlmTarget } from '@/lib/api/schemas'

export type ChapterTranslationForm = {
  providerId?: string
  target?: LlmTarget
  targetLanguage: string
  maxTokens: number
  brief: string
  batching: boolean
  batchSize: number
}

type ChapterTranslationState = ChapterTranslationForm & {
  operationId?: string
  startedPageCount?: number
  setForm: (patch: Partial<ChapterTranslationForm>) => void
  started: (operationId: string, pageCount: number) => void
  resetRun: () => void
}

export const useChapterTranslationStore = create<ChapterTranslationState>((set) => ({
  providerId: undefined,
  target: undefined,
  targetLanguage: 'zh-CN',
  maxTokens: 32000,
  brief: '',
  batching: false,
  batchSize: 50,
  operationId: undefined,
  startedPageCount: undefined,
  setForm: (patch) => set(patch),
  started: (operationId, startedPageCount) => set({ operationId, startedPageCount }),
  resetRun: () => set({ operationId: undefined, startedPageCount: undefined }),
}))
