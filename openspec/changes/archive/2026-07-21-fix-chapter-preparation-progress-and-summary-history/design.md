## Context

准备流程目前按 Artifact 阶段依次对全章运行，`prepared_page_count` 只有在页面同时拥有分割、气泡、字体预测和 OCR 后才增长，因此耗时的前置阶段无法形成页级反馈。批量翻译当前仅使用单个 `context_summary`，每次用户继续都会覆盖上一值；事件也只暴露一个 `batchSummary`。

## Goals / Non-Goals

**Goals:**
- 保证准备进度的每次增长都对应一个真实完成准备的页面。
- 保持已有 Artifact 缓存复用，不重新运行已经完成的步骤。
- 让后续批次获得按顺序排列的全部已确认摘要。
- 在审核 UI 中明确区分只读历史摘要与可编辑当前摘要。

**Non-Goals:**
- 不把一次 LLM 请求拆成逐页请求或伪造 token 级进度。
- 不允许修改已经确认并用于后续翻译的历史摘要。
- 不改变模型返回的 `translations` 与当前批 `batchSummary` 结构。

## Decisions

1. `prepare_pages` 按项目页序遍历，每页动态选择尚未就绪的准备引擎，并一次性运行该页的缺失链路。相比按阶段折算“等效页数”，这能让 `X / N` 始终表示真实完成页面，同时保留 Artifact 缓存。
2. 用 `Vec<String>` 保存已确认摘要；LLM 用户请求从单个 `contextSummary` 改为有序的 `contextSummaries`。相比拼接成一个字符串，结构化数组能保留批次边界并简化测试。
3. `PipelineProgress` 新增可选 `batchSummaries`，等待审核时包含“历史已确认摘要 + 当前生成摘要”。当前项始终为最后一项，继续接口仍只提交当前编辑值，避免客户端覆盖历史。
4. UI 使用现有 Accordion 展示历史摘要，每批一个只读折叠项；当前摘要继续使用 Textarea。翻译卡片改用状态文本和批次计数，不使用页数进度条。

## Risks / Trade-offs

- [准备调度从阶段优先改为页面优先，模型调用顺序发生变化] → 引擎实例仍由 Registry 缓存，且每页只选择缺失 Artifact，避免重复加载和重复计算。
- [摘要列表随批次数增长会增加输入长度] → 这是完整继承语义的必要成本；摘要仍由提示词要求保持简短，并受用户审核控制。
- [旧运行中的 SSE 事件没有 `batchSummaries`] → 字段保持可选，UI 回退到原 `batchSummary`，无需迁移持久数据。
