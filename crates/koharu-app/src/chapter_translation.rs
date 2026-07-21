use std::collections::{HashMap, HashSet};
use std::ops::Range;

use anyhow::{Context, Result, bail};
use koharu_core::{NodeDataPatch, NodeId, NodeKind, NodePatch, Op, PageId, Scene, TextDataPatch};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct TranslationUnit {
    pub id: String,
    pub page_id: PageId,
    pub page_index: usize,
    pub page_name: String,
    pub node_id: NodeId,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct ChapterSnapshot {
    pub page_ids: Vec<PageId>,
    pub units: Vec<TranslationUnit>,
}

impl ChapterSnapshot {
    pub fn from_scene(scene: &Scene) -> Result<Self> {
        let mut units = Vec::new();
        let mut page_ids = Vec::with_capacity(scene.pages.len());
        for (page_index, (page_id, page)) in scene.pages.iter().enumerate() {
            page_ids.push(*page_id);
            for (node_id, node) in &page.nodes {
                let NodeKind::Text(text) = &node.kind else {
                    continue;
                };
                let Some(source) = text.text.as_deref().map(str::trim) else {
                    bail!("page {} contains text without OCR output", page_index + 1);
                };
                if source.is_empty() {
                    bail!("page {} contains empty OCR output", page_index + 1);
                }
                units.push(TranslationUnit {
                    id: format!("t{}", units.len() + 1),
                    page_id: *page_id,
                    page_index,
                    page_name: page.name.clone(),
                    node_id: *node_id,
                    source: source.to_string(),
                });
            }
        }
        Ok(Self { page_ids, units })
    }

    pub fn units_for_pages(&self, page_range: Range<usize>) -> Vec<TranslationUnit> {
        self.units
            .iter()
            .filter(|unit| page_range.contains(&unit.page_index))
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedBatch {
    pub translations: Vec<String>,
    pub batch_summary: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BatchRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    context_summaries: Option<&'a [String]>,
    pages: Vec<RequestPage<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestPage<'a> {
    page_index: usize,
    page_name: &'a str,
    blocks: Vec<RequestBlock<'a>>,
}

#[derive(Serialize)]
struct RequestBlock<'a> {
    id: &'a str,
    source: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BatchResponse {
    translations: Vec<ResponseTranslation>,
    #[serde(default)]
    batch_summary: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResponseTranslation {
    id: String,
    text: String,
}

pub fn plan_batches(total_pages: usize, batch_size: Option<usize>) -> Vec<Range<usize>> {
    if total_pages == 0 {
        return Vec::new();
    }
    let size = batch_size
        .filter(|size| *size > 0 && *size < total_pages)
        .unwrap_or(total_pages);
    (0..total_pages)
        .step_by(size)
        .map(|start| start..(start + size).min(total_pages))
        .collect()
}

pub fn build_user_request(
    units: &[TranslationUnit],
    context_summaries: &[String],
) -> Result<String> {
    let mut pages: Vec<RequestPage<'_>> = Vec::new();
    for unit in units {
        if pages
            .last()
            .is_none_or(|page| page.page_index != unit.page_index)
        {
            pages.push(RequestPage {
                page_index: unit.page_index + 1,
                page_name: &unit.page_name,
                blocks: Vec::new(),
            });
        }
        pages
            .last_mut()
            .expect("page was inserted")
            .blocks
            .push(RequestBlock {
                id: &unit.id,
                source: &unit.source,
            });
    }
    serde_json::to_string(&BatchRequest {
        context_summaries: (!context_summaries.is_empty()).then_some(context_summaries),
        pages,
    })
    .map_err(Into::into)
}

pub fn fixed_system_prompt(target_language: &str, brief: Option<&str>, batching: bool) -> String {
    let (response_format, summary_instruction) = if batching {
        (
            r#"{"translations":[{"id":"t1","text":"译文"}],"batchSummary":"本批翻译要点和下一批需要保留的上下文"}"#,
            "同时返回 batchSummary，用简短文字总结本批翻译要点和下一批值得保留的上下文。只能根据输入原文和本批译文总结，不得编造。",
        )
    } else {
        (r#"{"translations":[{"id":"t1","text":"译文"}]}"#, "")
    };
    format!(
        "请将输入的漫画原文翻译成 {target_language}。contextSummaries 是用户按批次审核确认的全部历史摘要，已按时间顺序排列，仅用于保持本批翻译连贯；没有该字段时直接翻译当前内容。请保留 pages 中每个文本块的 id，只返回 JSON，不要添加解释、Markdown 或其他内容。返回格式必须是：{response_format}。translations 必须包含当前批次的每个 id 且仅出现一次，译文不能为空。{summary_instruction}\n用户提供的作品背景和术语要求：\n{}",
        brief.unwrap_or_default()
    )
}

pub fn parse_response(
    raw: &str,
    expected: &[TranslationUnit],
    require_batch_summary: bool,
) -> Result<ValidatedBatch> {
    let json = strip_json_fence(raw)?;
    let response: BatchResponse =
        serde_json::from_str(json).context("LLM response is not valid contract JSON")?;
    let expected_ids: HashSet<&str> = expected.iter().map(|unit| unit.id.as_str()).collect();
    let mut translated = HashMap::with_capacity(response.translations.len());
    for item in response.translations {
        if !expected_ids.contains(item.id.as_str()) {
            bail!("LLM response contains unknown id {}", item.id);
        }
        if item.text.trim().is_empty() {
            bail!("LLM response contains empty translation for {}", item.id);
        }
        if translated.insert(item.id.clone(), item.text).is_some() {
            bail!("LLM response contains duplicate id {}", item.id);
        }
    }
    if translated.len() != expected.len() {
        bail!("LLM response is missing one or more translations");
    }
    let translations = expected
        .iter()
        .map(|unit| translated.remove(&unit.id).expect("validated id set"))
        .collect();
    let batch_summary = match (require_batch_summary, response.batch_summary) {
        (true, Some(summary)) if !summary.trim().is_empty() => Some(summary.trim().to_string()),
        (true, _) => bail!("LLM batch response is missing a non-empty batchSummary"),
        (false, Some(_)) => bail!("LLM non-batch response must not contain batchSummary"),
        (false, None) => None,
    };
    Ok(ValidatedBatch {
        translations,
        batch_summary,
    })
}

fn strip_json_fence(raw: &str) -> Result<&str> {
    let trimmed = raw.trim();
    if let Some(inner) = trimmed.strip_prefix("```json") {
        let Some(inner) = inner.strip_suffix("```") else {
            bail!("unterminated JSON code fence");
        };
        if inner.contains("```") {
            bail!("nested code fence is not allowed");
        }
        return Ok(inner.trim());
    }
    if trimmed.starts_with("```") || trimmed.ends_with("```") {
        bail!("only a single JSON code fence is allowed");
    }
    Ok(trimmed)
}

pub fn translation_op(
    current: &Scene,
    units: &[TranslationUnit],
    validated: &ValidatedBatch,
    batch_index: usize,
) -> Result<Op> {
    if units.len() != validated.translations.len() {
        bail!("validated translation count no longer matches batch");
    }
    let mut ops = Vec::with_capacity(units.len());
    for (unit, translation) in units.iter().zip(&validated.translations) {
        let node = current
            .node(unit.page_id, unit.node_id)
            .with_context(|| format!("translation target {} no longer exists", unit.id))?;
        let NodeKind::Text(text) = &node.kind else {
            bail!("translation target {} is no longer a text node", unit.id);
        };
        if text.text.as_deref().map(str::trim) != Some(unit.source.as_str()) {
            bail!("OCR source changed while translating {}", unit.id);
        }
        let mut style = text.style.clone().unwrap_or_default();
        style.color = [0, 0, 0, 255];
        ops.push(Op::UpdateNode {
            page: unit.page_id,
            id: unit.node_id,
            patch: NodePatch {
                data: Some(NodeDataPatch::Text(TextDataPatch {
                    translation: Some(Some(translation.clone())),
                    style: Some(Some(style)),
                    ..Default::default()
                })),
                ..Default::default()
            },
            prev: NodePatch::default(),
        });
    }
    Ok(Op::Batch {
        ops,
        label: format!("chapter translation: batch {}", batch_index + 1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use koharu_core::{Node, Page, TextData, TextStyle, Transform};
    use serde_json::json;

    fn unit(id: &str) -> TranslationUnit {
        TranslationUnit {
            id: id.to_string(),
            page_id: PageId::new(),
            page_index: 0,
            page_name: "page".to_string(),
            node_id: NodeId::new(),
            source: format!("source-{id}"),
        }
    }

    fn valid_response(ids: &[&str], batch_summary: Option<&str>) -> String {
        let mut response = json!({
            "translations": ids
                .iter()
                .map(|id| json!({ "id": id, "text": format!("translated-{id}") }))
                .collect::<Vec<_>>()
        });
        if let Some(summary) = batch_summary {
            response["batchSummary"] = json!(summary);
        }
        response.to_string()
    }

    #[test]
    fn uses_a_minimal_chinese_prompt_and_response_contract() {
        let prompt = fixed_system_prompt("zh-CN", Some("固定译名：勇者"), false);
        assert!(prompt.contains("请将输入的漫画原文翻译成 zh-CN"));
        assert!(prompt.contains("固定译名：勇者"));
        assert!(prompt.contains(r#"{"translations":[{"id":"t1","text":"译文"}]}"#));
        assert!(!prompt.contains("storySummary"));
        assert!(!prompt.contains("relationships"));

        assert!(parse_response(&valid_response(&["a"], None), &[unit("a")], false).is_ok());

        let batch_prompt = fixed_system_prompt("zh-CN", None, true);
        assert!(batch_prompt.contains("同时返回 batchSummary"));
        let batch = parse_response(
            &valid_response(&["a"], Some("保持勇者的称谓")),
            &[unit("a")],
            true,
        )
        .unwrap();
        assert_eq!(batch.batch_summary.as_deref(), Some("保持勇者的称谓"));
        assert!(parse_response(&valid_response(&["a"], None), &[unit("a")], true).is_err());
    }

    #[test]
    fn plans_single_or_fixed_page_batches() {
        assert_eq!(plan_batches(300, None), vec![0..300]);
        assert_eq!(plan_batches(300, Some(300)), vec![0..300]);
        assert_eq!(plan_batches(300, Some(50)).len(), 6);
        assert_eq!(plan_batches(300, Some(50))[5], 250..300);
    }

    #[test]
    fn batch_request_includes_all_confirmed_summaries_in_order_without_previous_text() {
        let summaries = vec!["第一批确认摘要".to_string(), "第二批确认摘要".to_string()];
        let request = build_user_request(&[unit("b3-1")], &summaries).unwrap();
        let value: serde_json::Value = serde_json::from_str(&request).unwrap();
        assert_eq!(
            value["contextSummaries"],
            json!(["第一批确认摘要", "第二批确认摘要"])
        );
        assert_eq!(value["pages"][0]["blocks"][0]["id"], "b3-1");
        assert!(value.get("rollingContext").is_none());
        assert!(value.get("previousBatch").is_none());
    }

    #[test]
    fn retry_reuses_the_exact_context_summary() {
        let units = [unit("b3-1")];
        let summaries = vec!["第一批".to_string(), "第二批".to_string()];
        assert_eq!(
            build_user_request(&units, &summaries).unwrap(),
            build_user_request(&units, &summaries).unwrap()
        );
    }

    #[test]
    fn strictly_validates_translation_ids_and_text() {
        let expected = [unit("a"), unit("b")];
        assert!(parse_response(&valid_response(&["a", "b"], None), &expected, false).is_ok());

        for invalid in [
            valid_response(&["a"], None),
            valid_response(&["a", "a"], None),
            valid_response(&["a", "unknown"], None),
            valid_response(&["a", "b"], None).replace("translated-b", "   "),
            format!("explanation {}", valid_response(&["a", "b"], None)),
        ] {
            assert!(
                parse_response(&invalid, &expected, false).is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn accepts_one_json_fence_and_rejects_extra_fields() {
        let expected = [unit("a")];
        let fenced = format!("```json\n{}\n```", valid_response(&["a"], None));
        assert!(parse_response(&fenced, &expected, false).is_ok());

        let extra_context = json!({
            "translations": [{ "id": "a", "text": "translated" }],
            "nextContext": {}
        })
        .to_string();
        assert!(parse_response(&extra_context, &expected, false).is_err());
    }

    #[test]
    fn translation_write_forces_black_and_preserves_other_style_fields() {
        let mut scene = Scene::default();
        let mut page = Page::new("1", 100, 100);
        let page_id = page.id;
        let node_id = NodeId::new();
        page.nodes.insert(
            node_id,
            Node {
                id: node_id,
                transform: Transform::default(),
                visible: true,
                kind: NodeKind::Text(TextData {
                    text: Some("original".to_string()),
                    style: Some(TextStyle {
                        font_families: vec!["Noto Sans SC".to_string()],
                        font_size: Some(24.0),
                        color: [200, 100, 50, 255],
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
            },
        );
        scene.pages.insert(page_id, page);
        let expected = [TranslationUnit {
            id: "a".to_string(),
            page_id,
            page_index: 0,
            page_name: "1".to_string(),
            node_id,
            source: "original".to_string(),
        }];
        let validated = parse_response(&valid_response(&["a"], None), &expected, false).unwrap();

        let Op::Batch { ops, .. } = translation_op(&scene, &expected, &validated, 0).unwrap()
        else {
            panic!("expected a batch operation")
        };
        let Op::UpdateNode { patch, .. } = &ops[0] else {
            panic!("expected a text update")
        };
        let Some(NodeDataPatch::Text(text_patch)) = &patch.data else {
            panic!("expected a text patch")
        };
        let Some(Some(style)) = &text_patch.style else {
            panic!("chapter translation must persist an explicit style")
        };
        assert_eq!(style.color, [0, 0, 0, 255]);
        assert_eq!(style.font_families, vec!["Noto Sans SC"]);
        assert_eq!(style.font_size, Some(24.0));
    }

    #[test]
    fn refuses_atomic_write_when_ocr_source_changed() {
        let mut scene = Scene::default();
        let mut page = Page::new("1", 100, 100);
        let page_id = page.id;
        let node_id = NodeId::new();
        page.nodes.insert(
            node_id,
            Node {
                id: node_id,
                transform: Transform::default(),
                visible: true,
                kind: NodeKind::Text(TextData {
                    text: Some("original".to_string()),
                    ..Default::default()
                }),
            },
        );
        scene.pages.insert(page_id, page);
        let expected = [TranslationUnit {
            id: "a".to_string(),
            page_id,
            page_index: 0,
            page_name: "1".to_string(),
            node_id,
            source: "original".to_string(),
        }];
        let validated = parse_response(&valid_response(&["a"], None), &expected, false).unwrap();

        if let NodeKind::Text(text) = &mut scene
            .page_mut(page_id)
            .unwrap()
            .nodes
            .get_mut(&node_id)
            .unwrap()
            .kind
        {
            text.text = Some("edited".to_string());
        }
        assert!(translation_op(&scene, &expected, &validated, 0).is_err());
        let NodeKind::Text(text) = &scene.node(page_id, node_id).unwrap().kind else {
            unreachable!()
        };
        assert!(text.translation.is_none());
    }
}
