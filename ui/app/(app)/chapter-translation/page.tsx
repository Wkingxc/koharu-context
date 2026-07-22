'use client'

import {
  AlertCircleIcon,
  CheckCircle2Icon,
  LanguagesIcon,
  LoaderCircleIcon,
  PaintbrushIcon,
  ScanTextIcon,
} from 'lucide-react'
import Link from 'next/link'
import { useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { LlmModelSelect, type LlmModelOption } from '@/components/ui/llm-model-select'
import { Progress } from '@/components/ui/progress'
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'
import { useScene } from '@/hooks/useScene'
import {
  continueChapterTranslation,
  retryChapterTranslation,
  startChapterTranslation,
  useGetCatalog,
  useGetCurrentLlm,
} from '@/lib/api/default/default'
import type { LlmTarget } from '@/lib/api/schemas'
import { useChapterTranslationStore } from '@/lib/stores/chapterTranslationStore'
import { useEditorUiStore } from '@/lib/stores/editorUiStore'
import { useJobsStore } from '@/lib/stores/jobsStore'
import { usePreferencesStore } from '@/lib/stores/preferencesStore'

const ALLOWED_PROVIDERS = new Set(['openai', 'openai-compatible', 'claude', 'gemini', 'deepseek'])

const targetKey = (target: LlmTarget) =>
  `${target.kind}:${target.providerId ?? ''}:${target.modelId}`

export default function ChapterTranslationPage() {
  const { t } = useTranslation()
  const { scene } = useScene()
  const pageCount = scene ? Object.keys(scene.pages).length : 0
  const form = useChapterTranslationStore()
  const favoriteModels = usePreferencesStore((state) => state.favoriteModels)
  const toggleFavoriteModel = usePreferencesStore((state) => state.toggleFavoriteModel)
  const [starting, setStarting] = useState(false)
  const [startError, setStartError] = useState<string>()
  const editorTargetInheritanceResolved = useRef(false)
  const job = useJobsStore((state) => (form.operationId ? state.jobs[form.operationId] : undefined))
  const { data: catalog, isFetched: catalogFetched } = useGetCatalog({
    query: { staleTime: 30_000 },
  })
  const { data: currentLlm, isFetched: currentLlmFetched } = useGetCurrentLlm({
    query: { staleTime: 30_000 },
  })

  const providers = useMemo(
    () =>
      (catalog?.providers ?? []).filter(
        (provider) => ALLOWED_PROVIDERS.has(provider.id) && provider.status === 'ready',
      ),
    [catalog],
  )
  const selectedProvider = providers.find((provider) => provider.id === form.providerId)
  const modelOptions = useMemo<LlmModelOption[]>(
    () =>
      (selectedProvider?.models ?? []).map((model) => ({
        model,
        provider: selectedProvider,
      })),
    [selectedProvider],
  )
  const selectedModel = modelOptions.find(
    ({ model }) => form.target && targetKey(model.target) === targetKey(form.target),
  )?.model

  useEffect(() => {
    if (editorTargetInheritanceResolved.current || !catalogFetched || !currentLlmFetched) {
      return
    }
    editorTargetInheritanceResolved.current = true

    const editorTarget = currentLlm?.status === 'ready' ? currentLlm.target : undefined
    if (editorTarget?.kind !== 'provider' || !editorTarget.providerId) return
    const provider = providers.find((candidate) => candidate.id === editorTarget.providerId)
    const model = provider?.models.find(
      (candidate) => targetKey(candidate.target) === targetKey(editorTarget),
    )
    if (!provider || !model) return

    const currentForm = useChapterTranslationStore.getState()
    currentForm.setForm({
      providerId: provider.id,
      target: model.target,
      targetLanguage: model.languages.includes(currentForm.targetLanguage)
        ? currentForm.targetLanguage
        : (model.languages[0] ?? 'zh-CN'),
    })
  }, [catalogFetched, currentLlm, currentLlmFetched, providers])

  useEffect(() => {
    const currentForm = useChapterTranslationStore.getState()
    const provider =
      providers.find((candidate) => candidate.id === currentForm.providerId) ?? providers[0]
    if (!provider) return
    const target =
      provider.models.find(
        (model) => currentForm.target && targetKey(model.target) === targetKey(currentForm.target),
      )?.target ?? provider.models[0]?.target
    if (!target) return
    const model = provider.models.find(
      (candidate) => targetKey(candidate.target) === targetKey(target),
    )
    const language = model?.languages.includes(currentForm.targetLanguage)
      ? currentForm.targetLanguage
      : (model?.languages[0] ?? 'zh-CN')
    if (
      currentForm.providerId !== provider.id ||
      !currentForm.target ||
      targetKey(currentForm.target) !== targetKey(target) ||
      currentForm.targetLanguage !== language
    ) {
      currentForm.setForm({ providerId: provider.id, target, targetLanguage: language })
    }
  }, [form, providers])

  if (!scene) {
    return (
      <EmptyState
        title={t('chapterTranslation.noProject')}
        description={t('chapterTranslation.noProjectDescription')}
        action={t('chapterTranslation.returnToEditor')}
      />
    )
  }

  const isRunning = starting || job?.status === 'running'
  const isComplete = job?.status === 'completed' || job?.status === 'completed_with_errors'
  const failed = job?.status === 'failed' || job?.status === 'cancelled'

  const selectProvider = (providerId: string) => {
    const provider = providers.find((candidate) => candidate.id === providerId)
    const model = provider?.models[0]
    form.setForm({
      providerId,
      target: model?.target,
      targetLanguage: model?.languages.includes(form.targetLanguage)
        ? form.targetLanguage
        : (model?.languages[0] ?? 'zh-CN'),
    })
  }

  const selectModel = (key: string) => {
    const model = modelOptions.find((option) => targetKey(option.model.target) === key)?.model
    if (!model) return
    form.setForm({
      target: model.target,
      targetLanguage: model.languages.includes(form.targetLanguage)
        ? form.targetLanguage
        : (model.languages[0] ?? 'zh-CN'),
    })
  }

  const begin = async () => {
    if (!form.target || isRunning) return
    setStarting(true)
    setStartError(undefined)
    try {
      const response = await startChapterTranslation({
        target: form.target,
        targetLanguage: form.targetLanguage,
        maxTokens: form.maxTokens,
        brief: form.brief.trim() || undefined,
        batchSize: form.batching ? form.batchSize : undefined,
        defaultFont: usePreferencesStore.getState().defaultFont,
      })
      form.started(response.operationId, pageCount)
      useJobsStore.getState().started(response.operationId, 'chapter-translation')
    } catch (error) {
      const message = String(error)
      setStartError(message)
      useEditorUiStore.getState().showError(message)
    } finally {
      setStarting(false)
    }
  }

  const retryCurrentBatch = async () => {
    if (!form.operationId || !form.target || isRunning) return
    setStarting(true)
    setStartError(undefined)
    try {
      const response = await retryChapterTranslation(form.operationId, {
        target: form.target,
        maxTokens: form.maxTokens,
      })
      form.started(response.operationId, form.startedPageCount ?? pageCount)
      useJobsStore.getState().started(response.operationId, 'chapter-translation')
    } catch (error) {
      setStartError(String(error))
    } finally {
      setStarting(false)
    }
  }

  if (isComplete) {
    return (
      <main className='flex min-h-0 flex-1 items-center justify-center overflow-auto bg-muted/20 p-8'>
        <Card className='w-full max-w-2xl'>
          <CardHeader className='items-center text-center'>
            <CheckCircle2Icon className='mb-2 size-10 text-emerald-500' />
            <CardTitle>{t('chapterTranslation.completedTitle')}</CardTitle>
            <CardDescription>
              {t('chapterTranslation.completedDescription', {
                pages: form.startedPageCount ?? pageCount,
              })}
            </CardDescription>
          </CardHeader>
          <CardContent className='flex justify-center'>
            <Button asChild onClick={form.resetRun}>
              <Link href='/'>{t('chapterTranslation.returnToEditor')}</Link>
            </Button>
          </CardContent>
          {job?.error ? (
            <p className='px-6 pb-6 text-center text-sm text-amber-600'>{job.error}</p>
          ) : null}
        </Card>
      </main>
    )
  }

  return (
    <main className='min-h-0 flex-1 overflow-auto bg-muted/20 px-5 py-8 sm:px-8 lg:px-10'>
      <div className='mx-auto flex w-full max-w-6xl flex-col gap-7'>
        <div className='px-1'>
          <h1 className='text-2xl font-semibold'>{t('chapterTranslation.title')}</h1>
          <p className='mt-1 text-sm text-muted-foreground'>
            {t('chapterTranslation.projectSummary', {
              name: scene.project.name || t('chapterTranslation.untitled'),
              pages: pageCount,
            })}
          </p>
        </div>

        {isRunning ? (
          <RunningState job={job} totalPages={form.startedPageCount ?? pageCount} />
        ) : (
          <>
            <Card
              data-testid='chapter-translation-settings-card'
              className='gap-0 overflow-visible py-0'
            >
              <CardHeader className='gap-2 border-b px-6 py-6 sm:px-8'>
                <CardTitle>{t('chapterTranslation.configuration')}</CardTitle>
                <CardDescription>
                  {t('chapterTranslation.configurationDescription')}
                </CardDescription>
              </CardHeader>
              <CardContent className='grid gap-x-8 gap-y-7 px-6 py-7 sm:px-8 md:grid-cols-2'>
                <Field label={t('chapterTranslation.provider')}>
                  <select
                    data-testid='chapter-provider'
                    className='h-10 w-full rounded-md border border-input bg-background px-3 text-sm'
                    value={form.providerId ?? ''}
                    onChange={(event) => selectProvider(event.target.value)}
                  >
                    {providers.map((provider) => (
                      <option key={provider.id} value={provider.id}>
                        {provider.name}
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label={t('chapterTranslation.model')}>
                  <LlmModelSelect
                    data-testid='chapter-model'
                    value={form.target ? targetKey(form.target) : undefined}
                    options={modelOptions}
                    getKey={({ model }) => targetKey(model.target)}
                    onChange={selectModel}
                    placeholder={t('chapterTranslation.selectModel')}
                    triggerClassName='h-10 px-3 text-sm'
                    favoriteKeys={favoriteModels}
                    favoriteLabel={t('chapterTranslation.favoriteModel')}
                    unfavoriteLabel={t('chapterTranslation.unfavoriteModel')}
                    onToggleFavorite={toggleFavoriteModel}
                  />
                </Field>
                <Field label={t('chapterTranslation.targetLanguage')}>
                  <select
                    data-testid='chapter-language'
                    className='h-10 w-full rounded-md border border-input bg-background px-3 text-sm'
                    value={form.targetLanguage}
                    onChange={(event) => form.setForm({ targetLanguage: event.target.value })}
                  >
                    {(selectedModel?.languages ?? []).map((language) => (
                      <option key={language} value={language}>
                        {t(`llm.languages.${language}`, { defaultValue: language })}
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label={t('chapterTranslation.maxTokens')}>
                  <Input
                    data-testid='chapter-max-tokens'
                    type='number'
                    min={1}
                    className='h-10'
                    value={form.maxTokens || ''}
                    onChange={(event) =>
                      form.setForm({ maxTokens: Number(event.target.value) || 0 })
                    }
                  />
                </Field>
                <div className='md:col-span-2'>
                  <Field label={t('chapterTranslation.brief')}>
                    <Textarea
                      data-testid='chapter-brief'
                      rows={5}
                      className='min-h-32 resize-y'
                      value={form.brief}
                      placeholder={t('chapterTranslation.briefPlaceholder')}
                      onChange={(event) => form.setForm({ brief: event.target.value })}
                    />
                  </Field>
                </div>
              </CardContent>
            </Card>

            <Card data-testid='chapter-batching-card' className='gap-0 py-0'>
              <CardHeader className='flex flex-col gap-4 px-6 py-5 sm:flex-row sm:items-center sm:justify-between sm:px-8'>
                <div className='space-y-1.5'>
                  <Label htmlFor='chapter-batching' className='text-base font-semibold'>
                    {t('chapterTranslation.batching')}
                  </Label>
                  <CardDescription className='max-w-3xl'>
                    {t('chapterTranslation.batchingDescription')}
                  </CardDescription>
                </div>
                <Switch
                  id='chapter-batching'
                  data-testid='chapter-batching'
                  className='shrink-0'
                  checked={form.batching}
                  onCheckedChange={(batching) => form.setForm({ batching })}
                />
              </CardHeader>
              {form.batching ? (
                <CardContent className='border-t px-6 py-6 sm:px-8'>
                  <div className='max-w-sm'>
                    <Field label={t('chapterTranslation.batchSize')}>
                      <Input
                        data-testid='chapter-batch-size'
                        type='number'
                        min={1}
                        className='h-10'
                        value={form.batchSize || ''}
                        onChange={(event) =>
                          form.setForm({ batchSize: Number(event.target.value) || 0 })
                        }
                      />
                    </Field>
                  </div>
                </CardContent>
              ) : null}
            </Card>

            <div className='flex flex-col gap-4 px-1 pb-2'>
              {providers.length === 0 ? (
                <div className='rounded-lg border border-destructive/20 bg-destructive/5 p-3 text-sm text-destructive'>
                  {t('chapterTranslation.noProvider')}
                </div>
              ) : null}
              {startError || failed ? (
                <div className='flex flex-col gap-3 rounded-lg border border-destructive/20 bg-destructive/5 p-3 text-sm text-destructive sm:flex-row sm:items-center sm:justify-between'>
                  <div className='flex gap-2'>
                    <AlertCircleIcon className='mt-0.5 size-4 shrink-0' />
                    <span>{startError ?? job?.error ?? t('chapterTranslation.failed')}</span>
                  </div>
                  {job?.status === 'failed' && job.error?.includes('batch ') ? (
                    <Button
                      data-testid='chapter-retry-batch'
                      variant='outline'
                      size='sm'
                      className='self-end sm:self-auto'
                      onClick={() => void retryCurrentBatch()}
                    >
                      {t('chapterTranslation.retryBatch')}
                    </Button>
                  ) : failed ? (
                    <Button
                      data-testid='chapter-return-after-failure'
                      variant='outline'
                      size='sm'
                      className='self-end sm:self-auto'
                      asChild
                    >
                      <Link href='/'>{t('chapterTranslation.fixInEditor')}</Link>
                    </Button>
                  ) : null}
                </div>
              ) : null}
              <div className='flex flex-col-reverse gap-3 sm:flex-row sm:items-center sm:justify-end'>
                <Button variant='ghost' className='w-full sm:w-auto' asChild>
                  <Link href='/'>{t('common.cancel')}</Link>
                </Button>
                <Button
                  data-testid='chapter-start'
                  className='w-full sm:w-auto'
                  disabled={
                    !form.target ||
                    providers.length === 0 ||
                    starting ||
                    form.maxTokens < 1 ||
                    (form.batching && form.batchSize < 1)
                  }
                  onClick={() => void begin()}
                >
                  {starting ? <LoaderCircleIcon className='animate-spin' /> : null}
                  {t('chapterTranslation.start')}
                </Button>
              </div>
            </div>
          </>
        )}
      </div>
    </main>
  )
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className='grid gap-2'>
      <Label>{label}</Label>
      {children}
    </div>
  )
}

function RunningState({
  job,
  totalPages,
}: {
  job: ReturnType<typeof useJobsStore.getState>['jobs'][string] | undefined
  totalPages: number
}) {
  const { t } = useTranslation()
  const progress = job?.progress
  const phase = progress?.chapterPhase ?? 'preparing'
  const awaitingBatchReview = progress?.awaitingBatchReview === true
  const reviewSummaries =
    progress?.batchSummaries ?? (progress?.batchSummary ? [progress.batchSummary] : [])
  const historySummaries = reviewSummaries.slice(0, -1)
  const generatedSummary = reviewSummaries.at(-1) ?? ''
  const [summary, setSummary] = useState('')
  const [continuing, setContinuing] = useState(false)
  const [submitted, setSubmitted] = useState(false)
  const [continueError, setContinueError] = useState<string>()

  useEffect(() => {
    if (!awaitingBatchReview) return
    setSummary(generatedSummary)
    setSubmitted(false)
    setContinueError(undefined)
  }, [awaitingBatchReview, generatedSummary, progress?.currentBatch])

  const overallPercent = progress
    ? Math.min(100, Math.max(0, Math.round(progress.overallPercent)))
    : undefined
  const hasBatchProgress = Boolean(
    progress?.currentBatch && progress.totalBatches && progress.totalBatches > 1,
  )
  const currentStep = awaitingBatchReview
    ? t('chapterTranslation.reviewSummaryTitle')
    : progress?.step
      ? t(`chapterTranslation.steps.${progress.step}`)
      : t('chapterTranslation.preparing')
  const phases = [
    { id: 'preparing', icon: ScanTextIcon },
    { id: 'translating', icon: LanguagesIcon },
    { id: 'post_processing', icon: PaintbrushIcon },
  ] as const

  const chapterTotalPages = Math.max(0, progress?.chapterTotalPages ?? totalPages)
  const phaseCompletedPages = (item: (typeof phases)[number]['id']) => {
    const value =
      item === 'preparing'
        ? progress?.preparedPages
        : item === 'translating'
          ? progress?.translatedPages
          : progress?.renderedPages
    return Math.min(chapterTotalPages, Math.max(0, value ?? 0))
  }

  const phaseStatus = (item: (typeof phases)[number]['id']) => {
    if (chapterTotalPages > 0 && phaseCompletedPages(item) >= chapterTotalPages) return 'completed'
    if (item === phase && !awaitingBatchReview) return 'active'
    return 'waiting'
  }

  const continueBatch = async () => {
    if (!job || continuing || submitted) return
    setContinuing(true)
    setContinueError(undefined)
    try {
      await continueChapterTranslation(job.id, { summary })
      setSubmitted(true)
    } catch (error) {
      setContinueError(String(error))
    } finally {
      setContinuing(false)
    }
  }

  return (
    <Card className='gap-0 overflow-hidden py-0'>
      <CardHeader className='gap-5 border-b px-6 py-6 sm:px-8'>
        <div className='flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between'>
          <div className='min-w-0 space-y-2'>
            <CardTitle>{t('chapterTranslation.runningTitle')}</CardTitle>
            <CardDescription className='flex items-center gap-2 text-sm'>
              {awaitingBatchReview ? (
                <CheckCircle2Icon className='size-4 shrink-0 text-emerald-500' />
              ) : (
                <LoaderCircleIcon className='size-4 shrink-0 animate-spin text-primary' />
              )}
              <span>{currentStep}</span>
            </CardDescription>
          </div>
          <div className='flex items-baseline gap-2 sm:flex-col sm:items-end sm:gap-0'>
            <span className='text-sm text-muted-foreground'>
              {t('chapterTranslation.overallProgress')}
            </span>
            <span className='text-3xl font-semibold tabular-nums'>
              {overallPercent === undefined ? '—' : `${overallPercent}%`}
            </span>
          </div>
        </div>

        <div className='space-y-3'>
          {overallPercent === undefined ? (
            <div
              data-testid='chapter-overall-progress'
              role='progressbar'
              aria-label={t('chapterTranslation.overallProgress')}
              className='relative h-2.5 overflow-hidden rounded-full bg-primary/15'
            >
              <div className='activity-progress-indeterminate absolute inset-y-0 w-1/2 rounded-full bg-primary' />
            </div>
          ) : (
            <Progress
              data-testid='chapter-overall-progress'
              value={overallPercent}
              aria-label={t('chapterTranslation.overallProgress')}
              aria-valuenow={overallPercent}
              className='h-2.5'
            />
          )}
          <div className='flex min-h-7 flex-wrap gap-2'>
            {hasBatchProgress ? (
              <div className='rounded-md bg-muted px-2.5 py-1 text-xs text-muted-foreground'>
                {t('chapterTranslation.batchProgressLabel')}{' '}
                <span
                  data-testid='chapter-current-batch'
                  className='font-medium text-foreground tabular-nums'
                >
                  {progress!.currentBatch} / {progress!.totalBatches}
                </span>
              </div>
            ) : null}
          </div>
        </div>
      </CardHeader>
      <CardContent className='grid gap-4 px-6 py-6 sm:px-8 lg:grid-cols-3'>
        {phases.map((item) => {
          const status = phaseStatus(item.id)
          const active = status === 'active'
          const complete = status === 'completed'
          const Icon = item.icon
          const completedPages = phaseCompletedPages(item.id)
          const itemPercent =
            chapterTotalPages > 0 ? Math.round((completedPages / chapterTotalPages) * 100) : 0
          return (
            <div
              key={item.id}
              data-testid={`chapter-phase-${item.id}`}
              data-state={status}
              className={`rounded-xl border p-4 transition-colors ${
                active ? 'border-primary/70 bg-primary/5 shadow-sm' : 'bg-card'
              }`}
            >
              <div className='flex items-start justify-between gap-3'>
                <div className='flex min-w-0 items-center gap-3'>
                  <div
                    className={`flex size-9 shrink-0 items-center justify-center rounded-lg ${
                      active
                        ? 'bg-primary text-primary-foreground'
                        : complete
                          ? 'bg-emerald-500/10 text-emerald-600'
                          : 'bg-muted text-muted-foreground'
                    }`}
                  >
                    {complete ? (
                      <CheckCircle2Icon className='size-4' />
                    ) : (
                      <Icon className='size-4' />
                    )}
                  </div>
                  <div className='min-w-0'>
                    <div className='truncate text-sm font-medium'>
                      {t(`chapterTranslation.phases.${item.id}`)}
                    </div>
                    <div className='mt-0.5 text-xs text-muted-foreground'>
                      {t(`chapterTranslation.status.${status}`)}
                    </div>
                  </div>
                </div>
                {active ? (
                  <LoaderCircleIcon className='mt-1 size-4 shrink-0 animate-spin text-primary' />
                ) : null}
              </div>
              {item.id === 'translating' ? (
                <div
                  data-testid='chapter-translation-batch-status'
                  className='mt-4 flex h-5 items-center text-xs text-muted-foreground tabular-nums'
                >
                  {hasBatchProgress
                    ? `${t('chapterTranslation.batchProgressLabel')} ${progress!.currentBatch} / ${progress!.totalBatches}`
                    : t(`chapterTranslation.status.${status}`)}
                </div>
              ) : (
                <div className='mt-4 flex items-center gap-3'>
                  <Progress
                    value={itemPercent}
                    aria-label={t(`chapterTranslation.phases.${item.id}`)}
                    aria-valuenow={itemPercent}
                    className='h-1.5'
                  />
                  <span
                    data-testid={`chapter-phase-count-${item.id}`}
                    className='min-w-16 text-right text-xs text-muted-foreground tabular-nums'
                  >
                    {completedPages} / {chapterTotalPages}
                  </span>
                </div>
              )}
            </div>
          )
        })}
        {awaitingBatchReview ? (
          <div className='rounded-xl border border-primary/30 bg-primary/5 p-5 lg:col-span-3'>
            <div className='flex flex-col gap-5 lg:flex-row lg:items-end'>
              <div className='min-w-0 flex-1 space-y-2'>
                {historySummaries.length > 0 ? (
                  <div className='mb-5 space-y-2'>
                    <Label className='text-sm font-semibold'>
                      {t('chapterTranslation.summaryHistory')}
                    </Label>
                    <Accordion type='multiple' className='rounded-lg border bg-background px-4'>
                      {historySummaries.map((item, index) => (
                        <AccordionItem key={index} value={`batch-${index + 1}`}>
                          <AccordionTrigger
                            data-testid={`chapter-summary-history-trigger-${index + 1}`}
                            className='py-3 text-sm hover:no-underline'
                          >
                            {t('chapterTranslation.batchSummaryTitle', { batch: index + 1 })}
                          </AccordionTrigger>
                          <AccordionContent
                            data-testid={`chapter-summary-history-content-${index + 1}`}
                            className='text-sm whitespace-pre-wrap text-muted-foreground'
                          >
                            {item || t('chapterTranslation.emptyBatchSummary')}
                          </AccordionContent>
                        </AccordionItem>
                      ))}
                    </Accordion>
                  </div>
                ) : null}
                <Label htmlFor='chapter-batch-summary' className='text-sm font-semibold'>
                  {t('chapterTranslation.currentBatchSummary', {
                    batch: progress?.currentBatch,
                  })}
                </Label>
                <p className='text-sm text-muted-foreground'>
                  {t('chapterTranslation.reviewSummaryDescription')}
                </p>
                <Textarea
                  id='chapter-batch-summary'
                  data-testid='chapter-batch-summary'
                  rows={5}
                  className='min-h-28 resize-y bg-background'
                  value={summary}
                  placeholder={t('chapterTranslation.batchSummaryPlaceholder')}
                  onChange={(event) => setSummary(event.target.value)}
                />
                {continueError ? (
                  <p className='flex items-center gap-2 text-sm text-destructive'>
                    <AlertCircleIcon className='size-4 shrink-0' />
                    {t('chapterTranslation.continueBatchFailed')}
                  </p>
                ) : null}
              </div>
              <Button
                data-testid='chapter-continue-batch'
                className='w-full lg:w-auto'
                disabled={continuing || submitted}
                onClick={() => void continueBatch()}
              >
                {continuing ? <LoaderCircleIcon className='animate-spin' /> : null}
                {continuing || submitted
                  ? t('chapterTranslation.continuingBatch')
                  : t('chapterTranslation.continueBatch')}
              </Button>
            </div>
          </div>
        ) : null}
      </CardContent>
    </Card>
  )
}

function EmptyState({
  title,
  description,
  action,
}: {
  title: string
  description: string
  action: string
}) {
  return (
    <main className='flex min-h-0 flex-1 items-center justify-center p-8'>
      <Card className='w-full max-w-lg text-center'>
        <CardHeader>
          <CardTitle>{title}</CardTitle>
          <CardDescription>{description}</CardDescription>
        </CardHeader>
        <CardContent>
          <Button asChild>
            <Link href='/'>{action}</Link>
          </Button>
        </CardContent>
      </Card>
    </main>
  )
}
