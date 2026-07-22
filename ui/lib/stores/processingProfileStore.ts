'use client'

import { create } from 'zustand'
import { persist } from 'zustand/middleware'

import {
  deleteCurrentLlm,
  getConfig,
  getCurrentLlm,
  getGetCurrentLlmQueryKey,
  patchConfig,
  putCurrentLlm,
} from '@/lib/api/default/default'
import type { LlmTarget, PipelineConfig } from '@/lib/api/schemas'
import { queryClient } from '@/lib/queryClient'
import {
  useChapterTranslationStore,
  type ChapterTranslationForm,
} from '@/lib/stores/chapterTranslationStore'
import { useEditorUiStore } from '@/lib/stores/editorUiStore'
import { usePreferencesStore } from '@/lib/stores/preferencesStore'
import type { RenderEffect, RenderStroke } from '@/lib/types'

type CustomPipeline = {
  detect: boolean
  ocr: boolean
  translator: boolean
  inpainter: boolean
  renderer: boolean
}

export type ProcessingProfile = {
  id: string
  name: string
  createdAt: string
  pipeline: PipelineConfig
  selectedTarget?: LlmTarget
  selectedLanguage?: string
  readingOrder: 'rtl' | 'ltr' | 'custom'
  renderEffect: RenderEffect
  renderStroke?: RenderStroke
  defaultFont?: string
  customSystemPrompt?: string
  codexImagePrompt?: string
  codexImageModel?: string
  customPipeline: CustomPipeline
  chapterTranslation: ChapterTranslationForm
}

type ProcessingProfileState = {
  profiles: ProcessingProfile[]
  activeProfileId?: string
  addProfile: (profile: ProcessingProfile) => void
  deleteProfile: (id: string) => void
  setActiveProfile: (id?: string) => void
}

export const useProcessingProfileStore = create<ProcessingProfileState>()(
  persist(
    (set) => ({
      profiles: [],
      activeProfileId: undefined,
      addProfile: (profile) =>
        set((state) => ({ profiles: [...state.profiles, profile], activeProfileId: profile.id })),
      deleteProfile: (id) =>
        set((state) => ({
          profiles: state.profiles.filter((profile) => profile.id !== id),
          activeProfileId: state.activeProfileId === id ? undefined : state.activeProfileId,
        })),
      setActiveProfile: (activeProfileId) => set({ activeProfileId }),
    }),
    {
      name: 'koharu-processing-profiles',
      version: 1,
      partialize: (state) => ({
        profiles: state.profiles,
        activeProfileId: state.activeProfileId,
      }),
    },
  ),
)

const newProfileId = () =>
  globalThis.crypto?.randomUUID?.() ??
  `profile-${Date.now()}-${Math.random().toString(16).slice(2)}`

export async function captureProcessingProfile(name: string): Promise<ProcessingProfile> {
  const [config, llm] = await Promise.all([getConfig(), getCurrentLlm()])
  const preferences = usePreferencesStore.getState()
  const editor = useEditorUiStore.getState()
  const chapter = useChapterTranslationStore.getState()

  return {
    id: newProfileId(),
    name: name.trim(),
    createdAt: new Date().toISOString(),
    pipeline: { ...config.pipeline },
    selectedTarget: llm.target ?? editor.selectedTarget,
    selectedLanguage: editor.selectedLanguage,
    readingOrder: editor.readingOrder,
    renderEffect: { ...editor.renderEffect },
    renderStroke: editor.renderStroke ? { ...editor.renderStroke } : undefined,
    defaultFont: preferences.defaultFont,
    customSystemPrompt: preferences.customSystemPrompt,
    codexImagePrompt: preferences.codexImagePrompt,
    codexImageModel: preferences.codexImageModel,
    customPipeline: { ...preferences.customPipeline },
    chapterTranslation: {
      providerId: chapter.providerId,
      target: chapter.target,
      targetLanguage: chapter.targetLanguage,
      maxTokens: chapter.maxTokens,
      brief: chapter.brief,
      batching: chapter.batching,
      batchSize: chapter.batchSize,
    },
  }
}

export async function applyProcessingProfile(profile: ProcessingProfile): Promise<void> {
  await patchConfig({
    pipeline: {
      detector: profile.pipeline.detector,
      fontDetector: profile.pipeline.font_detector,
      segmenter: profile.pipeline.segmenter,
      bubbleSegmenter: profile.pipeline.bubble_segmenter,
      ocr: profile.pipeline.ocr,
      translator: profile.pipeline.translator,
      inpainter: profile.pipeline.inpainter,
      renderer: profile.pipeline.renderer,
    },
  })

  if (profile.selectedTarget) {
    await putCurrentLlm({ target: profile.selectedTarget })
  } else {
    await deleteCurrentLlm()
  }
  await queryClient.invalidateQueries({ queryKey: getGetCurrentLlmQueryKey() })

  usePreferencesStore.setState({
    defaultFont: profile.defaultFont,
    customSystemPrompt: profile.customSystemPrompt,
    codexImagePrompt: profile.codexImagePrompt,
    codexImageModel: profile.codexImageModel,
    customPipeline: { ...profile.customPipeline },
  })
  useEditorUiStore.setState({
    selectedTarget: profile.selectedTarget,
    selectedLanguage: profile.selectedLanguage,
    readingOrder: profile.readingOrder,
    renderEffect: { ...profile.renderEffect },
    renderStroke: profile.renderStroke ? { ...profile.renderStroke } : undefined,
  })
  useChapterTranslationStore.getState().setForm({ ...profile.chapterTranslation })
  useProcessingProfileStore.getState().setActiveProfile(profile.id)
}
