import { http, HttpResponse } from 'msw'
import { beforeEach, describe, expect, it } from 'vitest'

import { useChapterTranslationStore } from '@/lib/stores/chapterTranslationStore'
import { useEditorUiStore } from '@/lib/stores/editorUiStore'
import { usePreferencesStore } from '@/lib/stores/preferencesStore'
import {
  applyProcessingProfile,
  captureProcessingProfile,
  updateActiveProcessingProfile,
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
  it('provides the Japanese and Korean presets for a fresh install with Japanese active', () => {
    const initial = useProcessingProfileStore.getInitialState()

    expect(initial.profiles.map((profile) => profile.name)).toEqual(['日漫', '韩语'])
    expect(initial.activeProfileId).toBe(initial.profiles[0]?.id)
    expect(initial.profiles[0]).toMatchObject({
      pipeline: {
        detector: 'comic-text-bubble-detector',
        font_detector: 'yuzumarker-font-detection',
        segmenter: 'comic-text-detector-seg',
        bubble_segmenter: 'speech-bubble-segmentation',
        ocr: 'manga-ocr',
        translator: 'llm',
        inpainter: 'aot-inpainting',
        renderer: 'koharu-renderer',
      },
      selectedTarget: {
        kind: 'provider',
        providerId: 'openai-compatible',
        modelId: 'deepseek-v4-pro',
      },
      selectedLanguage: 'zh-CN',
      readingOrder: 'rtl',
      renderEffect: { italic: false, bold: false },
      defaultFont: 'Noto Sans SC:500',
      codexImageModel: 'gpt-5.5',
      customPipeline: {
        detect: true,
        ocr: true,
        translator: true,
        inpainter: true,
        renderer: true,
      },
      chapterTranslation: {
        targetLanguage: 'zh-CN',
        maxTokens: 32000,
        brief: '',
        batching: false,
        batchSize: 50,
      },
    })
    expect(initial.profiles[1]).toMatchObject({
      ...initial.profiles[0],
      id: 'builtin-korean-manga',
      name: '韩语',
      pipeline: {
        ...initial.profiles[0]?.pipeline,
        ocr: 'paddle-ocr-vl-1.6',
      },
    })
    expect(JSON.stringify(initial.profiles)).not.toContain('apiKey')
  })

  it('rehydrates existing profiles without appending or overwriting built-in presets', async () => {
    const existing = {
      ...useProcessingProfileStore.getInitialState().profiles[0]!,
      id: 'existing-profile',
      name: '我的配置',
      pipeline: {
        ...useProcessingProfileStore.getInitialState().profiles[0]!.pipeline,
        ocr: 'my-ocr',
      },
    }
    window.localStorage.setItem(
      'koharu-processing-profiles',
      JSON.stringify({
        state: { profiles: [existing], activeProfileId: existing.id },
        version: 1,
      }),
    )

    await useProcessingProfileStore.persist.rehydrate()

    expect(useProcessingProfileStore.getState().profiles).toEqual([existing])
    expect(useProcessingProfileStore.getState().activeProfileId).toBe(existing.id)
  })

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

  it('updates only the active profile while preserving its identity and position', async () => {
    let ocr = 'ocr-before'
    server.use(
      http.get('/api/v1/config', () =>
        HttpResponse.json({
          pipeline: {
            detector: 'detector-a',
            font_detector: 'font-a',
            segmenter: 'segmenter-a',
            bubble_segmenter: 'bubble-a',
            ocr,
            translator: 'llm',
            inpainter: 'inpaint-a',
            renderer: 'render-a',
          },
        }),
      ),
      http.get('/api/v1/llm/current', () => HttpResponse.json({ status: 'idle' })),
    )

    const active = await captureProcessingProfile('当前配置')
    const untouched = { ...active, id: 'untouched', name: '其他配置' }
    useProcessingProfileStore.setState({
      profiles: [active, untouched],
      activeProfileId: active.id,
    })

    ocr = 'ocr-after'
    usePreferencesStore.setState({ defaultFont: 'Updated Font' })
    const updated = await updateActiveProcessingProfile()
    const state = useProcessingProfileStore.getState()

    expect(updated?.id).toBe(active.id)
    expect(updated?.name).toBe(active.name)
    expect(updated?.createdAt).toBe(active.createdAt)
    expect(updated?.pipeline.ocr).toBe('ocr-after')
    expect(updated?.defaultFont).toBe('Updated Font')
    expect(state.profiles.map((profile) => profile.id)).toEqual([active.id, untouched.id])
    expect(state.profiles[1]).toEqual(untouched)
    expect(state.activeProfileId).toBe(active.id)
  })
})
