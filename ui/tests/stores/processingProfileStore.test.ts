import { http, HttpResponse } from 'msw'
import { beforeEach, describe, expect, it } from 'vitest'

import { useChapterTranslationStore } from '@/lib/stores/chapterTranslationStore'
import { useEditorUiStore } from '@/lib/stores/editorUiStore'
import { usePreferencesStore } from '@/lib/stores/preferencesStore'
import {
  applyProcessingProfile,
  captureProcessingProfile,
  useProcessingProfileStore,
} from '@/lib/stores/processingProfileStore'

import { server } from '../msw/server'

beforeEach(() => {
  window.localStorage.clear()
  useProcessingProfileStore.setState({ profiles: [], activeProfileId: undefined })
  usePreferencesStore.setState({
    defaultFont: 'Noto Sans SC',
    customSystemPrompt: '术语提示',
    customPipeline: {
      detect: true,
      ocr: true,
      translator: true,
      inpainter: false,
      renderer: true,
    },
  })
  useEditorUiStore.setState({
    selectedLanguage: 'zh-CN',
    selectedTarget: undefined,
    readingOrder: 'rtl',
    renderEffect: { italic: false, bold: true },
  })
  useChapterTranslationStore.setState({
    providerId: 'openai',
    target: { kind: 'provider', providerId: 'openai', modelId: 'chapter-model' },
    targetLanguage: 'zh-CN',
    maxTokens: 32000,
    brief: '作品背景',
    batching: true,
    batchSize: 50,
  })
})

describe('processing profiles', () => {
  it('captures and applies manga processing settings without machine settings or API keys', async () => {
    const applied: unknown[] = []
    const loaded: unknown[] = []
    server.use(
      http.get('/api/v1/config', () =>
        HttpResponse.json({
          data: { path: '/machine-specific' },
          http: { connect_timeout: 20, read_timeout: 300, max_retries: 3 },
          pipeline: {
            detector: 'detector-a',
            font_detector: 'font-a',
            segmenter: 'segmenter-a',
            bubble_segmenter: 'bubble-a',
            ocr: 'ocr-a',
            translator: 'llm',
            inpainter: 'inpaint-a',
            renderer: 'render-a',
          },
          providers: [{ id: 'openai', api_key: '[REDACTED]' }],
        }),
      ),
      http.get('/api/v1/llm/current', () =>
        HttpResponse.json({
          status: 'ready',
          target: { kind: 'provider', providerId: 'openai', modelId: 'loaded-model' },
        }),
      ),
      http.patch('/api/v1/config', async ({ request }) => {
        applied.push(await request.json())
        return HttpResponse.json({})
      }),
      http.put('/api/v1/llm/current', async ({ request }) => {
        loaded.push(await request.json())
        return new HttpResponse(null, { status: 204 })
      }),
    )

    const profile = await captureProcessingProfile('黑白漫画')
    expect(profile.selectedTarget?.modelId).toBe('loaded-model')
    expect(profile.pipeline.ocr).toBe('ocr-a')
    expect(profile.chapterTranslation.brief).toBe('作品背景')
    expect(profile.readingOrder).toBe('rtl')
    expect(profile.renderEffect.bold).toBe(true)
    expect(profile).not.toHaveProperty('data')
    expect(profile).not.toHaveProperty('http')
    expect(profile).not.toHaveProperty('providers')

    useProcessingProfileStore.getState().addProfile(profile)
    useEditorUiStore.setState({ selectedLanguage: 'en-US', readingOrder: 'ltr' })
    await applyProcessingProfile(profile)

    expect(applied).toEqual([
      {
        pipeline: {
          detector: 'detector-a',
          fontDetector: 'font-a',
          segmenter: 'segmenter-a',
          bubbleSegmenter: 'bubble-a',
          ocr: 'ocr-a',
          translator: 'llm',
          inpainter: 'inpaint-a',
          renderer: 'render-a',
        },
      },
    ])
    expect(loaded).toEqual([{ target: profile.selectedTarget }])
    expect(useEditorUiStore.getState().selectedLanguage).toBe('zh-CN')
    expect(useEditorUiStore.getState().readingOrder).toBe('rtl')
    expect(usePreferencesStore.getState().defaultFont).toBe('Noto Sans SC')
    expect(useChapterTranslationStore.getState().batchSize).toBe(50)
    expect(useProcessingProfileStore.getState().activeProfileId).toBe(profile.id)
  })
})
