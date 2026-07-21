## 1. API LLM 任务参数

- [x] 1.1 让 `LlmLoadRequest.options.max_tokens` 在 OpenAI、OpenAI-compatible、Claude、Gemini 和 DeepSeek Provider 中端到端生效，同时保持未传值的现有调用行为，并用 Provider 请求序列化测试验证参数 | risk=medium | evidence=focused

## 2. 章节翻译后端闭环

- [x] 2.1 交付支持可选 `batchSize` 的 `POST /chapter-translations` 长任务入口、对话式 API Provider 后端校验和阶段/批次进度，按 `Artifact::ready` 仅为未就绪页面复用检测/OCR，并在准备失败时证明未调用翻译 API | risk=high | evidence=focused
- [x] 2.2 交付有页面边界的章节 JSON 快照、单次或串行分批调度、固定规则加用户 `brief`、第 N 批继承此前全部已确认摘要、严格响应校验和逐批原子 `Op::Batch` 写回，并覆盖不分批、300 页按 50 页分 6 批、摘要推进、当前批重试、缺失/重复/未知/空结果及并发原文变化场景 | risk=high | evidence=focused
- [x] 2.3 在每批译文成功写回后复用现有 inpainter 和 renderer 完成该批后处理，确认当前批完成后才进入下一批，正确发布成功、完成但有错误和失败状态，并验证现有单页流水线及 `.khr` 项目兼容性 | risk=medium | evidence=focused

## 3. App 内章节翻译工作区

- [x] 3.1 在顶部“处理”和“帮助”之间交付“章节翻译”直达入口、`/chapter-translation` 配置/运行/完成三态页面及包含可选 `batchSize` 和批次状态的会话 Store，连接生成后的 OpenAPI 客户端，并验证未打开项目、未点击开始、重复启动和路由返回状态 | risk=medium | evidence=focused
- [x] 3.2 交付 Provider/模型过滤、目标语言、最大输出 token、任务提示词、默认关闭的分批开关、每批页数、`第 N/M 批` 反馈、当前批失败提示和“返回编辑器精修”的完整交互与中英文文案，使用单批与多批项目完成端到端 UI 验证 | risk=medium | evidence=full
