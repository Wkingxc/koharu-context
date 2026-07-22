import { act, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { beforeEach, describe, expect, it } from 'vitest'

import ChapterTranslationPage from '@/app/(app)/chapter-translation/page'
import { getGetSceneJsonQueryKey } from '@/lib/api/default/default'
import { queryClient } from '@/lib/queryClient'
import { useChapterTranslationStore } from '@/lib/stores/chapterTranslationStore'
import { useJobsStore } from '@/lib/stores/jobsStore'
import { usePreferencesStore } from '@/lib/stores/preferencesStore'

import { renderWithQuery } from '../helpers'
import { server } from '../msw/server'

const sceneSnapshot = {
  epoch: 0,
  scene: {
    project: { name: 'Chapter One' },
    pages: {
      p1: { id: 'p1', name: '1', width: 100, height: 100, nodes: {} },
      p2: { id: 'p2', name: '2', width: 100, height: 100, nodes: {} },
    },
  },
}

beforeEach(() => {
  queryClient.clear()
  window.localStorage.clear()
  useJobsStore.getState().clear()
  usePreferencesStore.setState({ defaultFont: undefined })
  useChapterTranslationStore.setState({
    providerId: undefined,
    target: undefined,
    targetLanguage: 'zh-CN',
    maxTokens: useChapterTranslationStore.getInitialState().maxTokens,
    brief: '',
    batching: false,
    batchSize: 50,
    operationId: undefined,
    startedPageCount: undefined,
  })
  queryClient.setQueryData(getGetSceneJsonQueryKey(), sceneSnapshot)
  server.use(
    http.get('/api/v1/scene.json', () => HttpResponse.json(sceneSnapshot)),
    http.get('/api/v1/llm/catalog', () =>
      HttpResponse.json({
        localModels: [],
        providers: [
          {
            id: 'openai',
            name: 'OpenAI',
            status: 'ready',
            requiresApiKey: true,
            requiresBaseUrl: false,
            hasApiKey: true,
            models: [
              {
                name: 'GPT Test',
                target: { kind: 'provider', providerId: 'openai', modelId: 'gpt-test' },
                languages: ['zh-CN', 'en-US'],
              },
              {
                name: 'GPT Pro',
                target: { kind: 'provider', providerId: 'openai', modelId: 'gpt-pro' },
                languages: ['zh-CN', 'en-US'],
              },
            ],
          },
        ],
      }),
    ),
  )
})

describe('ChapterTranslationPage', () => {
  it('shows a return state when no project is open', async () => {
    queryClient.clear()
    server.use(
      http.get('/api/v1/scene.json', () =>
        HttpResponse.json({ message: 'no project' }, { status: 400 }),
      ),
    )
    renderWithQuery(<ChapterTranslationPage />)
    expect(await screen.findByText('chapterTranslation.noProject')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'chapterTranslation.returnToEditor' })).toHaveAttribute(
      'href',
      '/',
    )
  })

  it('does not start before confirmation and sends the selected batch settings once', async () => {
    const requests: unknown[] = []
    server.use(
      http.post('/api/v1/chapter-translations', async ({ request }) => {
        requests.push(await request.json())
        return HttpResponse.json({ operationId: 'chapter-op' })
      }),
    )

    usePreferencesStore.setState({ defaultFont: 'Noto Sans SC' })
    renderWithQuery(<ChapterTranslationPage />)
    const start = await screen.findByTestId('chapter-start')
    expect(requests).toHaveLength(0)

    await userEvent.type(screen.getByTestId('chapter-brief'), 'Use the official glossary.')
    await userEvent.click(screen.getByTestId('chapter-batching'))
    const batchSize = await screen.findByTestId('chapter-batch-size')
    await userEvent.clear(batchSize)
    await userEvent.type(batchSize, '25')
    await userEvent.click(start)

    await waitFor(() => expect(requests).toHaveLength(1))
    expect(requests[0]).toMatchObject({
      target: { kind: 'provider', providerId: 'openai', modelId: 'gpt-test' },
      targetLanguage: 'zh-CN',
      maxTokens: 32000,
      batchSize: 25,
      brief: 'Use the official glossary.',
      defaultFont: 'Noto Sans SC',
    })
    expect(await screen.findByText('chapterTranslation.runningTitle')).toBeInTheDocument()
    expect(requests).toHaveLength(1)
  })

  it('favorites a model without selecting it and keeps favorites at the top', async () => {
    renderWithQuery(<ChapterTranslationPage />)
    const trigger = await screen.findByTestId('chapter-model')
    await waitFor(() => expect(trigger).toHaveTextContent('GPT Test'))

    await userEvent.click(trigger)
    const gptProOption = await screen.findByTitle('GPT Pro')
    await userEvent.click(
      within(gptProOption).getByRole('button', {
        name: 'chapterTranslation.favoriteModel GPT Pro',
      }),
    )

    expect(trigger).toHaveTextContent('GPT Test')
    const reorderedModels = screen
      .getAllByRole('option')
      .filter((option) => option.hasAttribute('title'))
    expect(reorderedModels[0]).toHaveTextContent('GPT Pro')
    const unfavorite = within(reorderedModels[0]).getByRole('button', {
      name: 'chapterTranslation.unfavoriteModel GPT Pro',
    })
    expect(unfavorite).toBeInTheDocument()
    expect(
      JSON.parse(window.localStorage.getItem('koharu-config') ?? '{}').state.favoriteModels,
    ).toContain('provider:openai:gpt-pro')

    await userEvent.click(unfavorite)
    expect(trigger).toHaveTextContent('GPT Test')
    expect(
      JSON.parse(window.localStorage.getItem('koharu-config') ?? '{}').state.favoriteModels,
    ).not.toContain('provider:openai:gpt-pro')
  })

  it('returns to editing without a duplicate export action and resets the completed run', async () => {
    useChapterTranslationStore.setState({ operationId: 'done-op', startedPageCount: 2 })
    useJobsStore.getState().started('done-op', 'chapter-translation')
    useJobsStore.getState().finished('done-op', 'completed', undefined)

    renderWithQuery(<ChapterTranslationPage />)
    expect(await screen.findByText('chapterTranslation.completedTitle')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'chapterTranslation.returnToEditor' })).toHaveAttribute(
      'href',
      '/',
    )
    expect(
      screen.queryByRole('button', { name: 'chapterTranslation.exportAll' }),
    ).not.toBeInTheDocument()

    await userEvent.click(screen.getByRole('link', { name: 'chapterTranslation.returnToEditor' }))
    expect(useChapterTranslationStore.getState().operationId).toBeUndefined()
    expect(await screen.findByTestId('chapter-translation-settings-card')).toBeInTheDocument()
  })

  it('shows live preparation, translation, and post-processing progress', async () => {
    useChapterTranslationStore.setState({ operationId: 'running-op', startedPageCount: 4 })
    useJobsStore.getState().started('running-op', 'chapter-translation')
    useJobsStore.getState().progress({
      jobId: 'running-op',
      status: { status: 'running' },
      chapterPhase: 'preparing',
      step: 'ocr',
      currentPage: 1,
      totalPages: 4,
      currentStepIndex: 0,
      totalSteps: 3,
      overallPercent: 12,
      currentBatch: null,
      totalBatches: null,
      chapterTotalPages: 100,
      preparedPages: 12,
      translatedPages: 0,
      renderedPages: 0,
    })

    renderWithQuery(<ChapterTranslationPage />)
    expect(await screen.findByTestId('chapter-overall-progress')).toHaveAttribute(
      'aria-valuenow',
      '12',
    )
    expect(screen.getByTestId('chapter-phase-preparing')).toHaveAttribute('data-state', 'active')
    expect(screen.getByTestId('chapter-phase-count-preparing')).toHaveTextContent('12 / 100')
    expect(screen.getByText('chapterTranslation.steps.ocr')).toBeInTheDocument()

    act(() => {
      useJobsStore.getState().progress({
        jobId: 'running-op',
        status: { status: 'running' },
        chapterPhase: 'translating',
        step: 'llmGenerate',
        currentPage: 1,
        totalPages: 3,
        currentStepIndex: 1,
        totalSteps: 3,
        overallPercent: 46,
        currentBatch: 2,
        totalBatches: 3,
        chapterTotalPages: 100,
        preparedPages: 100,
        translatedPages: 50,
        renderedPages: 25,
      })
    })
    expect(screen.getByTestId('chapter-phase-translating')).toHaveAttribute('data-state', 'active')
    expect(screen.getByTestId('chapter-current-batch')).toHaveTextContent('2 / 3')
    expect(screen.getByTestId('chapter-translation-batch-status')).toHaveTextContent('2 / 3')
    expect(screen.getByTestId('chapter-phase-count-preparing')).toHaveTextContent('100 / 100')
    expect(screen.queryByTestId('chapter-phase-count-translating')).not.toBeInTheDocument()
    expect(screen.getByTestId('chapter-phase-count-post_processing')).toHaveTextContent('25 / 100')

    act(() => {
      useJobsStore.getState().progress({
        jobId: 'running-op',
        status: { status: 'running' },
        chapterPhase: 'post_processing',
        step: 'render',
        currentPage: 3,
        totalPages: 4,
        currentStepIndex: 2,
        totalSteps: 3,
        overallPercent: 72,
        currentBatch: 2,
        totalBatches: 3,
        chapterTotalPages: 100,
        preparedPages: 100,
        translatedPages: 75,
        renderedPages: 40,
      })
    })
    expect(screen.getByTestId('chapter-phase-post_processing')).toHaveAttribute(
      'data-state',
      'active',
    )
    expect(screen.getByText('chapterTranslation.steps.render')).toBeInTheDocument()
    expect(screen.queryByTestId('chapter-phase-count-translating')).not.toBeInTheDocument()
    expect(screen.getByTestId('chapter-phase-count-post_processing')).toHaveTextContent('40 / 100')
  })

  it('shows collapsible read-only summary history and edits only the current batch', async () => {
    const continuations: unknown[] = []
    server.use(
      http.post('/api/v1/chapter-translations/review-op/continue', async ({ request }) => {
        continuations.push(await request.json())
        return new HttpResponse(null, { status: 204 })
      }),
    )
    useChapterTranslationStore.setState({ operationId: 'review-op', startedPageCount: 100 })
    useJobsStore.getState().started('review-op', 'chapter-translation')
    useJobsStore.getState().progress({
      jobId: 'review-op',
      status: { status: 'running' },
      chapterPhase: 'post_processing',
      step: null,
      currentPage: 49,
      totalPages: 50,
      currentStepIndex: 2,
      totalSteps: 3,
      overallPercent: 50,
      currentBatch: 3,
      totalBatches: 4,
      awaitingBatchReview: true,
      batchSummary: '第三批模型摘要',
      batchSummaries: ['第一批确认摘要', '第二批确认摘要', '第三批模型摘要'],
    })

    renderWithQuery(<ChapterTranslationPage />)
    const summary = await screen.findByTestId('chapter-batch-summary')
    expect(summary).toHaveValue('第三批模型摘要')
    expect(screen.getByTestId('chapter-summary-history-trigger-1')).toBeInTheDocument()
    expect(screen.getByTestId('chapter-summary-history-trigger-2')).toBeInTheDocument()
    expect(screen.queryByText('第一批确认摘要')).not.toBeInTheDocument()
    await userEvent.click(screen.getByTestId('chapter-summary-history-trigger-1'))
    expect(await screen.findByText('第一批确认摘要')).toBeInTheDocument()
    expect(continuations).toHaveLength(0)

    await userEvent.clear(summary)
    await userEvent.type(summary, '用户修改后的摘要')
    await userEvent.click(screen.getByTestId('chapter-continue-batch'))

    await waitFor(() => expect(continuations).toEqual([{ summary: '用户修改后的摘要' }]))
  })

  it('retries a failed batch through its preserved backend checkpoint', async () => {
    let retried = 0
    server.use(
      http.post('/api/v1/chapter-translations/failed-op/retry', () => {
        retried += 1
        return HttpResponse.json({ operationId: 'retry-op' })
      }),
    )
    useChapterTranslationStore.setState({ operationId: 'failed-op', startedPageCount: 2 })
    useJobsStore.getState().started('failed-op', 'chapter-translation')
    useJobsStore.getState().finished('failed-op', 'failed', 'batch 2 response validation failed')

    renderWithQuery(<ChapterTranslationPage />)
    await userEvent.click(await screen.findByTestId('chapter-retry-batch'))
    await waitFor(() => expect(retried).toBe(1))
    expect(useChapterTranslationStore.getState().operationId).toBe('retry-op')
    expect(useJobsStore.getState().jobs['retry-op']?.status).toBe('running')
  })

  it('offers a return-to-editor action when page preparation leaves empty OCR blocks', async () => {
    useChapterTranslationStore.setState({ operationId: 'ocr-failed', startedPageCount: 118 })
    useJobsStore.getState().started('ocr-failed', 'chapter-translation')
    useJobsStore
      .getState()
      .finished(
        'ocr-failed',
        'failed',
        'OCR output is still missing after preparation: page 118 "118.png" (text blocks: 1)',
      )

    renderWithQuery(<ChapterTranslationPage />)

    expect(await screen.findByText(/page 118/)).toBeInTheDocument()
    expect(screen.getByTestId('chapter-return-after-failure')).toHaveAttribute('href', '/')
  })
})
