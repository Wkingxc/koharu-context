//! Chapter-level API-LLM translation orchestration.

use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result, bail};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use dashmap::DashMap;
use koharu_app::ProjectSession;
use koharu_app::chapter_translation::{
    ChapterSnapshot, build_user_request, fixed_system_prompt, parse_response, plan_batches,
    translation_op,
};
use koharu_app::pipeline::{self, Artifact, PipelineRunOptions, PipelineSpec, Scope};
use koharu_core::{
    AppEvent, ChapterTranslationPhase, JobFinishedEvent, JobStatus, JobSummary, JobWarningEvent,
    LlmGenerationOptions, LlmTarget, LlmTargetKind, PipelineProgress, PipelineStatus, PipelineStep,
};
use koharu_llm::{Language, providers::build_provider};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::AppState;
use crate::error::{ApiError, ApiResult};
use crate::routes::operations::{register_cancel, unregister_cancel};

const ALLOWED_PROVIDERS: &[&str] = &[
    "openai",
    "openai-compatible",
    "claude",
    "gemini",
    "deepseek",
];

#[derive(Debug, Clone, Copy)]
struct ChapterPageCounts {
    total: usize,
    prepared: usize,
    translated: usize,
    rendered: usize,
}

impl ChapterPageCounts {
    fn overall_percent(self) -> u8 {
        let total_units = self.total.max(1) * 3;
        let completed = self.prepared.min(self.total)
            + self.translated.min(self.total)
            + self.rendered.min(self.total);
        ((completed * 100) / total_units).min(100) as u8
    }
}

#[derive(Debug, Clone, Copy)]
struct ChapterProgressScope {
    phase: ChapterTranslationPhase,
    current_batch: Option<usize>,
    total_batches: Option<usize>,
    pages: ChapterPageCounts,
}

impl ChapterProgressScope {
    fn with_prepared(mut self, prepared: usize) -> Self {
        self.pages.prepared = prepared.min(self.pages.total);
        self
    }

    fn with_translated(mut self, translated: usize) -> Self {
        self.pages.translated = translated.min(self.pages.total);
        self
    }

    fn with_rendered(mut self, rendered: usize) -> Self {
        self.pages.rendered = rendered.min(self.pages.total);
        self
    }

    fn event(
        self,
        operation_id: &str,
        step: Option<PipelineStep>,
        current_page: usize,
        total_pages: usize,
        _local_percent: u8,
    ) -> PipelineProgress {
        PipelineProgress {
            job_id: operation_id.to_string(),
            status: PipelineStatus::Running,
            step,
            current_page,
            total_pages: total_pages.max(1),
            current_step_index: match self.phase {
                ChapterTranslationPhase::Preparing => 0,
                ChapterTranslationPhase::Translating => 1,
                ChapterTranslationPhase::PostProcessing => 2,
            },
            total_steps: 3,
            overall_percent: self.pages.overall_percent(),
            chapter_phase: Some(self.phase),
            current_batch: self.current_batch,
            total_batches: self.total_batches,
            awaiting_batch_review: false,
            batch_summary: None,
            batch_summaries: None,
            chapter_total_pages: Some(self.pages.total),
            prepared_pages: Some(self.pages.prepared.min(self.pages.total)),
            translated_pages: Some(self.pages.translated.min(self.pages.total)),
            rendered_pages: Some(self.pages.rendered.min(self.pages.total)),
        }
    }
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::default()
        .routes(routes!(start_chapter_translation))
        .routes(routes!(retry_chapter_translation))
        .routes(routes!(continue_chapter_translation))
}

#[derive(Clone)]
struct RetryCheckpoint {
    request: StartChapterTranslationRequest,
    batch_index: usize,
    context_summaries: Vec<String>,
}

static RETRY_CHECKPOINTS: OnceLock<DashMap<String, RetryCheckpoint>> = OnceLock::new();

fn retry_checkpoints() -> &'static DashMap<String, RetryCheckpoint> {
    RETRY_CHECKPOINTS.get_or_init(DashMap::new)
}

static PENDING_BATCH_REVIEWS: OnceLock<DashMap<String, oneshot::Sender<String>>> = OnceLock::new();

fn pending_batch_reviews() -> &'static DashMap<String, oneshot::Sender<String>> {
    PENDING_BATCH_REVIEWS.get_or_init(DashMap::new)
}

fn register_pending_batch_review(operation_id: &str) -> oneshot::Receiver<String> {
    let (sender, receiver) = oneshot::channel();
    pending_batch_reviews().insert(operation_id.to_string(), sender);
    receiver
}

fn submit_batch_review(operation_id: &str, summary: String) -> Result<(), String> {
    let (_, sender) = pending_batch_reviews()
        .remove(operation_id)
        .ok_or_else(|| "chapter translation is not waiting for batch review".to_string())?;
    sender
        .send(summary)
        .map_err(|_| "chapter translation is no longer waiting for batch review".to_string())
}

pub(crate) fn cancel_pending_batch_review(operation_id: &str) {
    pending_batch_reviews().remove(operation_id);
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StartChapterTranslationRequest {
    pub target: LlmTarget,
    pub target_language: String,
    pub max_tokens: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brief: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_font: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StartChapterTranslationResponse {
    pub operation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContinueChapterTranslationRequest {
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RetryChapterTranslationRequest {
    pub target: LlmTarget,
    pub max_tokens: u32,
}

#[utoipa::path(
    post,
    path = "/chapter-translations",
    request_body = StartChapterTranslationRequest,
    responses((status = 200, body = StartChapterTranslationResponse))
)]
async fn start_chapter_translation(
    State(app): State<AppState>,
    Json(req): Json<StartChapterTranslationRequest>,
) -> ApiResult<Json<StartChapterTranslationResponse>> {
    validate_request(&req).map_err(|error| ApiError::bad_request(format!("{error:#}")))?;
    let session = app
        .current_session()
        .ok_or_else(|| ApiError::bad_request("no project open"))?;
    if session.scene.read().pages.is_empty() {
        return Err(ApiError::bad_request("project has no pages"));
    }
    if app
        .jobs
        .iter()
        .any(|job| job.kind == "chapter-translation" && job.status == JobStatus::Running)
    {
        return Err(ApiError::bad_request(
            "a chapter translation is already running",
        ));
    }

    let operation_id = spawn_operation(app, session, req, None);
    Ok(Json(StartChapterTranslationResponse { operation_id }))
}

#[utoipa::path(
    post,
    path = "/chapter-translations/{id}/retry",
    params(("id" = String, Path, description = "Failed chapter operation id")),
    request_body = RetryChapterTranslationRequest,
    responses((status = 200, body = StartChapterTranslationResponse))
)]
async fn retry_chapter_translation(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<RetryChapterTranslationRequest>,
) -> ApiResult<Json<StartChapterTranslationResponse>> {
    let (_, mut checkpoint) = retry_checkpoints()
        .remove(&id)
        .ok_or_else(|| ApiError::bad_request("chapter operation has no retry checkpoint"))?;
    checkpoint.request.target = req.target;
    checkpoint.request.max_tokens = req.max_tokens;
    if let Err(error) = validate_request(&checkpoint.request) {
        retry_checkpoints().insert(id, checkpoint);
        return Err(ApiError::bad_request(format!("{error:#}")));
    }
    let session = app
        .current_session()
        .ok_or_else(|| ApiError::bad_request("no project open"))?;
    if app
        .jobs
        .iter()
        .any(|job| job.kind == "chapter-translation" && job.status == JobStatus::Running)
    {
        retry_checkpoints().insert(id, checkpoint);
        return Err(ApiError::bad_request(
            "a chapter translation is already running",
        ));
    }
    let operation_id = spawn_operation(
        app,
        session,
        checkpoint.request,
        Some((checkpoint.batch_index, checkpoint.context_summaries)),
    );
    Ok(Json(StartChapterTranslationResponse { operation_id }))
}

#[utoipa::path(
    post,
    path = "/chapter-translations/{id}/continue",
    params(("id" = String, Path, description = "Chapter operation id")),
    request_body = ContinueChapterTranslationRequest,
    responses((status = 204))
)]
async fn continue_chapter_translation(
    Path(id): Path<String>,
    Json(req): Json<ContinueChapterTranslationRequest>,
) -> ApiResult<StatusCode> {
    submit_batch_review(&id, req.summary).map_err(ApiError::bad_request)?;
    Ok(StatusCode::NO_CONTENT)
}

fn spawn_operation(
    app: AppState,
    session: Arc<ProjectSession>,
    req: StartChapterTranslationRequest,
    resume: Option<(usize, Vec<String>)>,
) -> String {
    let operation_id = Uuid::new_v4().to_string();
    let cancel = Arc::new(AtomicBool::new(false));
    register_cancel(operation_id.clone(), cancel.clone());
    set_job(&app, &operation_id, JobStatus::Running, None);
    app.bus.publish(AppEvent::JobStarted {
        id: operation_id.clone(),
        kind: "chapter-translation".to_string(),
    });
    let app_c = app.clone();
    let operation_id_c = operation_id.clone();
    tokio::spawn(async move {
        let result = run_chapter_translation(
            app_c.clone(),
            session,
            operation_id_c.clone(),
            req,
            cancel,
            resume,
        )
        .await;
        cancel_pending_batch_review(&operation_id_c);
        let (status, error) = match result {
            Ok(None) => (JobStatus::Completed, None),
            Ok(Some(warning)) => (JobStatus::CompletedWithErrors, Some(warning)),
            Err(error) if error.to_string().contains("cancelled") => (JobStatus::Cancelled, None),
            Err(error) => {
                tracing::warn!(operation_id = %operation_id_c, "chapter translation failed: {error:#}");
                (JobStatus::Failed, Some(format!("{error:#}")))
            }
        };
        if matches!(status, JobStatus::Completed | JobStatus::Cancelled) {
            retry_checkpoints().remove(&operation_id_c);
        }
        set_job(&app_c, &operation_id_c, status, error.clone());
        app_c.bus.publish(AppEvent::JobFinished(JobFinishedEvent {
            id: operation_id_c.clone(),
            status,
            error,
        }));
        unregister_cancel(&operation_id_c);
    });
    operation_id
}

fn validate_request(req: &StartChapterTranslationRequest) -> Result<()> {
    if req.target.kind != LlmTargetKind::Provider {
        bail!("chapter translation requires an API provider model");
    }
    let provider = req
        .target
        .provider_id
        .as_deref()
        .context("provider target is missing providerId")?;
    if !ALLOWED_PROVIDERS.contains(&provider) {
        bail!("unsupported chapter translation provider: {provider}");
    }
    if req.target.model_id.trim().is_empty() {
        bail!("modelId is required");
    }
    if Language::parse(&req.target_language).is_none() {
        bail!("unsupported target language: {}", req.target_language);
    }
    if req.max_tokens == 0 {
        bail!("maxTokens must be a positive integer");
    }
    if req.batch_size == Some(0) {
        bail!("batchSize must be a positive integer when provided");
    }
    Ok(())
}

async fn run_chapter_translation(
    app: AppState,
    session: Arc<ProjectSession>,
    operation_id: String,
    req: StartChapterTranslationRequest,
    cancel: Arc<AtomicBool>,
    resume: Option<(usize, Vec<String>)>,
) -> Result<Option<String>> {
    let project_pages = session.scene.read().pages.len().max(1);
    let initial_prepared = prepared_page_count(&session.scene.read());
    let initial_counts = ChapterPageCounts {
        total: project_pages,
        prepared: initial_prepared,
        translated: 0,
        rendered: 0,
    };
    let preparing_scope = ChapterProgressScope {
        phase: ChapterTranslationPhase::Preparing,
        current_batch: None,
        total_batches: None,
        pages: initial_counts,
    };
    publish_progress(
        &app,
        &operation_id,
        preparing_scope,
        Some(PipelineStep::Detect),
        0,
        project_pages,
        0,
    );
    prepare_pages(
        &app,
        session.clone(),
        &operation_id,
        cancel.clone(),
        initial_counts,
    )
    .await?;
    ensure_not_cancelled(&cancel)?;
    publish_progress(
        &app,
        &operation_id,
        preparing_scope.with_prepared(project_pages),
        Some(PipelineStep::Ocr),
        project_pages.saturating_sub(1),
        project_pages,
        100,
    );

    let snapshot = ChapterSnapshot::from_scene(&session.scene_snapshot())?;
    let batches = plan_batches(snapshot.page_ids.len(), req.batch_size);
    let total_batches = batches.len();
    let provider_id = req
        .target
        .provider_id
        .as_deref()
        .expect("validated provider id");
    let options = LlmGenerationOptions {
        temperature: None,
        max_tokens: Some(req.max_tokens),
        custom_system_prompt: None,
    };
    let provider_config = koharu_app::llm::provider_config_from_settings(
        &app.config.load(),
        &app.runtime,
        provider_id,
        Some(&options),
    );
    // Chapter translation must stay bound to the target captured by this
    // operation. The editor's shared LLM can be changed independently (for
    // example by loading a processing profile), so using it here could send a
    // later batch to an unrelated model.
    let provider = build_provider(provider_id, provider_config)?;
    tracing::info!(
        operation_id,
        provider_id,
        model_id = %req.target.model_id,
        "chapter translation provider ready"
    );

    let language = Language::parse(&req.target_language).expect("validated language");
    let batching = req.batch_size.is_some();
    let system_prompt = fixed_system_prompt(&req.target_language, req.brief.as_deref(), batching);
    let (start_batch, mut context_summaries) = resume.unwrap_or_default();
    if start_batch >= total_batches {
        bail!("retry checkpoint batch is outside the current chapter");
    }

    for (batch_index, page_range) in batches.into_iter().enumerate().skip(start_batch) {
        ensure_not_cancelled(&cancel)?;
        let batch_number = batch_index + 1;
        let units = snapshot.units_for_pages(page_range.clone());
        let translating_scope = ChapterProgressScope {
            phase: ChapterTranslationPhase::Translating,
            current_batch: Some(batch_number),
            total_batches: Some(total_batches),
            pages: ChapterPageCounts {
                total: project_pages,
                prepared: project_pages,
                translated: page_range.start,
                rendered: page_range.start,
            },
        };
        retry_checkpoints().insert(
            operation_id.clone(),
            RetryCheckpoint {
                request: req.clone(),
                batch_index,
                context_summaries: context_summaries.clone(),
            },
        );
        publish_progress(
            &app,
            &operation_id,
            translating_scope,
            Some(PipelineStep::LlmGenerate),
            page_range.start.min(project_pages.saturating_sub(1)),
            project_pages,
            0,
        );

        let mut generated_summary = None;
        if !units.is_empty() {
            let user_request = build_user_request(&units, &context_summaries)?;
            let raw = provider
                .generate(
                    &user_request,
                    language,
                    &req.target.model_id,
                    &system_prompt,
                )
                .await
                .with_context(|| format!("batch {batch_number} LLM request failed"))?;
            let validated = parse_response(&raw, &units, batching)
                .with_context(|| format!("batch {batch_number} response validation failed"))?;
            generated_summary = validated.batch_summary.clone();
            let op = translation_op(&session.scene_snapshot(), &units, &validated, batch_index)?;
            session.apply(op)?;
        }

        publish_progress(
            &app,
            &operation_id,
            translating_scope.with_translated(page_range.end),
            Some(PipelineStep::LlmGenerate),
            page_range.end.saturating_sub(1),
            project_pages,
            100,
        );

        let post_processing_scope = ChapterProgressScope {
            phase: ChapterTranslationPhase::PostProcessing,
            current_batch: Some(batch_number),
            total_batches: Some(total_batches),
            pages: ChapterPageCounts {
                total: project_pages,
                prepared: project_pages,
                translated: page_range.end,
                rendered: page_range.start,
            },
        };
        publish_progress(
            &app,
            &operation_id,
            post_processing_scope,
            Some(PipelineStep::Inpaint),
            0,
            page_range.len(),
            0,
        );
        let pages = snapshot.page_ids[page_range.clone()].to_vec();
        let page_count = pages.len().max(1);
        let config = app.config.load().pipeline.clone();
        let warning_count = run_pipeline(
            &app,
            session.clone(),
            &operation_id,
            vec![config.inpainter, config.renderer],
            pages,
            cancel.clone(),
            Some(post_processing_scope),
            post_processing_options(&req),
        )
        .await?;
        let completed_post_processing = post_processing_scope.with_rendered(page_range.end);
        publish_progress(
            &app,
            &operation_id,
            completed_post_processing,
            Some(PipelineStep::Render),
            page_count.saturating_sub(1),
            page_count,
            100,
        );
        if warning_count > 0 {
            return Ok(Some(format!(
                "batch {batch_number} translations were saved, but {warning_count} post-processing step(s) failed"
            )));
        }
        if batching
            && batch_number < total_batches
            && let Some(summary) = generated_summary
        {
            let confirmed = wait_for_batch_review(
                &app,
                &operation_id,
                completed_post_processing,
                page_count,
                &context_summaries,
                summary,
                &cancel,
            )
            .await?;
            context_summaries.push(confirmed);
        }
    }

    retry_checkpoints().remove(&operation_id);
    Ok(None)
}

async fn wait_for_batch_review(
    app: &AppState,
    operation_id: &str,
    scope: ChapterProgressScope,
    page_count: usize,
    confirmed_summaries: &[String],
    summary: String,
    cancel: &AtomicBool,
) -> Result<String> {
    let receiver = register_pending_batch_review(operation_id);
    let mut progress = scope.event(
        operation_id,
        None,
        page_count.saturating_sub(1),
        page_count,
        100,
    );
    progress.awaiting_batch_review = true;
    progress.batch_summary = Some(summary.clone());
    let mut summaries = confirmed_summaries.to_vec();
    summaries.push(summary);
    progress.batch_summaries = Some(summaries);
    publish_job_progress(app, progress);

    match receiver.await {
        Ok(summary) => Ok(summary),
        Err(_) if cancel.load(Ordering::Relaxed) => bail!("cancelled"),
        Err(_) => bail!("batch review was interrupted"),
    }
}

async fn prepare_pages(
    app: &AppState,
    session: Arc<ProjectSession>,
    operation_id: &str,
    cancel: Arc<AtomicBool>,
    page_counts: ChapterPageCounts,
) -> Result<()> {
    let config = app.config.load().pipeline.clone();
    let stages = [
        (Artifact::TextBoxes, config.detector),
        (Artifact::SegmentMask, config.segmenter),
        (Artifact::BubbleMask, config.bubble_segmenter),
        (Artifact::FontPredictions, config.font_detector),
        (Artifact::OcrText, config.ocr),
    ];
    let page_ids = session
        .scene
        .read()
        .pages
        .keys()
        .copied()
        .collect::<Vec<_>>();
    for page_id in page_ids {
        ensure_not_cancelled(&cancel)?;
        let engines = {
            let scene = session.scene.read();
            let page = scene
                .pages
                .get(&page_id)
                .context("chapter page disappeared during preparation")?;
            missing_preparation_engines(page, &stages)
        };
        if engines.is_empty() {
            continue;
        }
        let scope = ChapterProgressScope {
            phase: ChapterTranslationPhase::Preparing,
            current_batch: None,
            total_batches: None,
            pages: page_counts,
        };
        let warnings = run_pipeline(
            app,
            session.clone(),
            operation_id,
            engines,
            vec![page_id],
            cancel.clone(),
            Some(scope),
            PipelineRunOptions::default(),
        )
        .await?;
        if warnings > 0 {
            bail!("page preparation failed before the translation API was called");
        }
    }
    let missing_ocr = {
        let scene = session.scene.read();
        missing_ocr_details(&scene)
    };
    if !missing_ocr.is_empty() {
        bail!(
            "OCR output is still missing after preparation: {}",
            missing_ocr.join("; ")
        );
    }
    Ok(())
}

fn missing_ocr_details(scene: &koharu_core::Scene) -> Vec<String> {
    scene
        .pages
        .values()
        .enumerate()
        .filter_map(|(page_index, page)| {
            let missing_blocks = page
                .nodes
                .values()
                .filter_map(|node| match &node.kind {
                    koharu_core::NodeKind::Text(text) => Some(text),
                    _ => None,
                })
                .enumerate()
                .filter_map(|(text_index, text)| {
                    (!text
                        .text
                        .as_ref()
                        .is_some_and(|value| !value.trim().is_empty()))
                    .then_some(text_index + 1)
                })
                .collect::<Vec<_>>();
            (!missing_blocks.is_empty()).then(|| {
                format!(
                    "page {} \"{}\" (text blocks: {})",
                    page_index + 1,
                    page.name,
                    missing_blocks
                        .iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
        })
        .collect()
}

fn missing_preparation_engines(
    page: &koharu_core::Page,
    stages: &[(Artifact, String)],
) -> Vec<String> {
    if Artifact::SegmentMask.ready(page)
        && Artifact::BubbleMask.ready(page)
        && Artifact::FontPredictions.ready(page)
        && Artifact::OcrText.ready(page)
    {
        return Vec::new();
    }
    let text_detection_pending = !Artifact::TextBoxes.ready(page);
    stages
        .iter()
        .filter_map(|(artifact, engine)| {
            let may_become_missing_after_detection = text_detection_pending
                && matches!(artifact, Artifact::FontPredictions | Artifact::OcrText);
            (!artifact.ready(page) || may_become_missing_after_detection).then_some(engine.clone())
        })
        .collect()
}

fn prepared_page_count(scene: &koharu_core::Scene) -> usize {
    scene
        .pages
        .values()
        .filter(|page| {
            Artifact::SegmentMask.ready(page)
                && Artifact::BubbleMask.ready(page)
                && Artifact::FontPredictions.ready(page)
                && Artifact::OcrText.ready(page)
        })
        .count()
}

fn completed_pages_for_pipeline_tick(
    page_index: usize,
    total_pages: usize,
    step: Option<PipelineStep>,
    overall_percent: u8,
) -> usize {
    if step.is_none() && overall_percent >= 100 {
        total_pages
    } else {
        page_index.min(total_pages)
    }
}

async fn run_pipeline(
    app: &AppState,
    session: Arc<ProjectSession>,
    operation_id: &str,
    steps: Vec<String>,
    pages: Vec<koharu_core::PageId>,
    cancel: Arc<AtomicBool>,
    progress_scope: Option<ChapterProgressScope>,
    options: PipelineRunOptions,
) -> Result<usize> {
    for step in &steps {
        pipeline::Registry::find(step)?;
    }
    let warning_bus = app.bus.clone();
    let warning_job = operation_id.to_string();
    let warning_sink: pipeline::WarningSink = Arc::new(move |tick| {
        warning_bus.publish(AppEvent::JobWarning(JobWarningEvent {
            job_id: warning_job.clone(),
            page_index: tick.page_index,
            total_pages: tick.total_pages,
            step_id: tick.step_id,
            message: tick.message,
        }));
    });
    let progress_sink = progress_scope.map(|scope| {
        let progress_app = app.clone();
        let progress_job = operation_id.to_string();
        let progress_session = session.clone();
        Arc::new(move |tick: pipeline::ProgressTick| {
            let current_scope = match scope.phase {
                ChapterTranslationPhase::Preparing => {
                    scope.with_prepared(prepared_page_count(&progress_session.scene.read()))
                }
                ChapterTranslationPhase::PostProcessing => {
                    let completed_in_batch = completed_pages_for_pipeline_tick(
                        tick.page_index,
                        tick.total_pages,
                        tick.step,
                        tick.overall_percent,
                    );
                    scope.with_rendered(scope.pages.rendered + completed_in_batch)
                }
                ChapterTranslationPhase::Translating => scope,
            };
            publish_job_progress(
                &progress_app,
                current_scope.event(
                    &progress_job,
                    tick.step,
                    tick.page_index,
                    tick.total_pages,
                    tick.overall_percent,
                ),
            );
        }) as pipeline::ProgressSink
    });
    let outcome = pipeline::run(
        session,
        app.registry.clone(),
        app.runtime.clone(),
        app.cpu_only(),
        app.llm.clone(),
        app.renderer.clone(),
        PipelineSpec {
            scope: Scope::Pages(pages),
            steps,
            options,
        },
        cancel,
        progress_sink,
        Some(warning_sink),
    )
    .await?;
    Ok(outcome.warning_count)
}

fn post_processing_options(req: &StartChapterTranslationRequest) -> PipelineRunOptions {
    PipelineRunOptions {
        target_language: Some(req.target_language.clone()),
        default_font: req.default_font.clone(),
        ..Default::default()
    }
}

fn ensure_not_cancelled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        bail!("cancelled");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn publish_progress(
    app: &AppState,
    operation_id: &str,
    scope: ChapterProgressScope,
    step: Option<PipelineStep>,
    current_page: usize,
    total_pages: usize,
    local_percent: u8,
) {
    publish_job_progress(
        app,
        scope.event(operation_id, step, current_page, total_pages, local_percent),
    );
}

fn publish_job_progress(app: &AppState, progress: PipelineProgress) {
    if let Some(mut job) = app.jobs.get_mut(&progress.job_id) {
        job.progress = Some(progress.clone());
    }
    app.bus.publish(AppEvent::JobProgress(progress));
}

fn set_job(app: &AppState, id: &str, status: JobStatus, error: Option<String>) {
    app.jobs.insert(
        id.to_string(),
        JobSummary {
            id: id.to_string(),
            kind: "chapter-translation".to_string(),
            status,
            error,
            progress: None,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use koharu_core::{
        BlobRef, MaskData, MaskRole, Node, NodeId, NodeKind, Page, Scene, TextData, Transform,
    };

    fn request(provider: &str) -> StartChapterTranslationRequest {
        StartChapterTranslationRequest {
            target: LlmTarget {
                kind: LlmTargetKind::Provider,
                model_id: "model".to_string(),
                provider_id: Some(provider.to_string()),
            },
            target_language: "zh-CN".to_string(),
            max_tokens: 8192,
            brief: None,
            batch_size: Some(50),
            default_font: None,
        }
    }

    #[test]
    fn post_processing_inherits_the_editor_default_font() {
        let mut req = request("openai");
        req.default_font = Some("Noto Sans SC".to_string());

        let options = post_processing_options(&req);

        assert_eq!(options.default_font.as_deref(), Some("Noto Sans SC"));
        assert_eq!(options.target_language.as_deref(), Some("zh-CN"));
    }

    #[test]
    fn accepts_only_conversational_api_providers() {
        for provider in ALLOWED_PROVIDERS {
            assert!(validate_request(&request(provider)).is_ok());
        }
        assert!(validate_request(&request("deepl")).is_err());
        let mut local = request("openai");
        local.target.kind = LlmTargetKind::Local;
        assert!(validate_request(&local).is_err());
    }

    #[test]
    fn reports_monotonic_image_counts_for_each_chapter_phase() {
        let event = ChapterProgressScope {
            phase: ChapterTranslationPhase::PostProcessing,
            current_batch: Some(2),
            total_batches: Some(6),
            pages: ChapterPageCounts {
                total: 300,
                prepared: 300,
                translated: 100,
                rendered: 50,
            },
        }
        .event("job", Some(PipelineStep::Render), 12, 50, 50);
        assert_eq!(
            event.chapter_phase,
            Some(ChapterTranslationPhase::PostProcessing)
        );
        assert_eq!(event.step, Some(PipelineStep::Render));
        assert_eq!((event.current_page, event.total_pages), (12, 50));
        assert_eq!(
            (event.current_batch, event.total_batches),
            (Some(2), Some(6))
        );
        assert_eq!(event.chapter_total_pages, Some(300));
        assert_eq!(event.prepared_pages, Some(300));
        assert_eq!(event.translated_pages, Some(100));
        assert_eq!(event.rendered_pages, Some(50));
        assert_eq!(event.overall_percent, 50);

        assert_eq!(
            completed_pages_for_pipeline_tick(12, 50, Some(PipelineStep::Render), 50),
            12
        );
        assert_eq!(completed_pages_for_pipeline_tick(49, 50, None, 100), 50);
    }

    #[test]
    fn prepared_count_includes_cached_empty_pages_but_not_unprocessed_pages() {
        let mut cached_empty_page = Page::new("cached-empty", 800, 1200);
        for (role, hash) in [
            (MaskRole::Segment, "segment-mask"),
            (MaskRole::Bubble, "bubble-mask"),
        ] {
            let id = NodeId::new();
            cached_empty_page.nodes.insert(
                id,
                Node {
                    id,
                    transform: Transform::default(),
                    visible: true,
                    kind: NodeKind::Mask(MaskData {
                        role,
                        blob: BlobRef::new(hash),
                    }),
                },
            );
        }
        let unprocessed_page = Page::new("unprocessed", 800, 1200);
        let mut scene = Scene::default();
        scene.pages.insert(cached_empty_page.id, cached_empty_page);
        scene.pages.insert(unprocessed_page.id, unprocessed_page);

        assert_eq!(prepared_page_count(&scene), 1);
    }

    #[test]
    fn preparation_plan_skips_cached_artifacts_for_each_page() {
        let fresh_page = Page::new("fresh", 800, 1200);
        let mut page = Page::new("partially-cached", 800, 1200);
        let text_id = NodeId::new();
        page.nodes.insert(
            text_id,
            Node {
                id: text_id,
                transform: Transform::default(),
                visible: true,
                kind: NodeKind::Text(TextData::default()),
            },
        );
        let mask_id = NodeId::new();
        page.nodes.insert(
            mask_id,
            Node {
                id: mask_id,
                transform: Transform::default(),
                visible: true,
                kind: NodeKind::Mask(MaskData {
                    role: MaskRole::Segment,
                    blob: BlobRef::new("segment-mask"),
                }),
            },
        );
        let stages = [
            (Artifact::TextBoxes, "detector".to_string()),
            (Artifact::SegmentMask, "segmenter".to_string()),
            (Artifact::BubbleMask, "bubble".to_string()),
            (Artifact::FontPredictions, "font".to_string()),
            (Artifact::OcrText, "ocr".to_string()),
        ];

        assert_eq!(
            missing_preparation_engines(&fresh_page, &stages),
            vec![
                "detector".to_string(),
                "segmenter".to_string(),
                "bubble".to_string(),
                "font".to_string(),
                "ocr".to_string(),
            ]
        );

        assert_eq!(
            missing_preparation_engines(&page, &stages),
            vec!["bubble".to_string(), "font".to_string(), "ocr".to_string()]
        );
    }

    #[test]
    fn missing_ocr_details_identifies_page_and_text_blocks_but_skips_empty_pages() {
        let empty_page = Page::new("blank.png", 800, 1200);
        let mut page = Page::new("page-002.png", 800, 1200);
        for text in [Some("ready".to_string()), Some("  ".to_string()), None] {
            let id = NodeId::new();
            page.nodes.insert(
                id,
                Node {
                    id,
                    transform: Transform::default(),
                    visible: true,
                    kind: NodeKind::Text(TextData {
                        text,
                        ..Default::default()
                    }),
                },
            );
        }
        let mut scene = Scene::default();
        scene.pages.insert(empty_page.id, empty_page);
        scene.pages.insert(page.id, page);

        assert_eq!(
            missing_ocr_details(&scene),
            vec!["page 2 \"page-002.png\" (text blocks: 2, 3)".to_string()]
        );
    }

    #[test]
    fn validates_positive_limits() {
        let mut req = request("openai");
        req.max_tokens = 0;
        assert!(validate_request(&req).is_err());
        req.max_tokens = 1;
        req.batch_size = Some(0);
        assert!(validate_request(&req).is_err());
    }

    #[tokio::test]
    async fn batch_review_continuation_is_one_shot_and_cancellable() {
        let operation_id = Uuid::new_v4().to_string();
        let receiver = register_pending_batch_review(&operation_id);
        assert!(submit_batch_review(&operation_id, "用户修改后的摘要".to_string()).is_ok());
        assert_eq!(receiver.await.unwrap(), "用户修改后的摘要");
        assert!(submit_batch_review(&operation_id, "重复提交".to_string()).is_err());

        let cancelled_id = Uuid::new_v4().to_string();
        let cancelled = register_pending_batch_review(&cancelled_id);
        cancel_pending_batch_review(&cancelled_id);
        assert!(cancelled.await.is_err());
    }
}
