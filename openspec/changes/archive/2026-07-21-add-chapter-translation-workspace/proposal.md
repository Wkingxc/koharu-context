## Why

当前整项目流水线按页面依次执行翻译，LLM 只能理解单页文本，容易造成跨页人物称谓、术语和语义不一致。需要增加一个面向整章的独立工作区：内容量合适时由 API LLM 一次翻译全部有序 OCR 文本，内容量较大时允许用户按固定页数分批，并由后一批继承此前全部经用户确认的摘要。

## What Changes

- 在顶部菜单栏增加与“处理”“帮助”同级的“章节翻译”直达入口，进入 App 内独立页面，而不是启动外部网页。
- 新增章节翻译工作区，用于确认当前项目范围、API Provider、模型、目标语言、最大输出 token、本次翻译提示词，以及是否分批和每批页数。
- 启动后复用现有检测与 OCR 引擎补齐全部页面文本；未启用分批时发送一次整章请求，启用分批时按项目页序切分并顺序执行多个请求。
- 分批模式中，第 N 批接收固定规则、用户提示词、此前全部经用户确认的摘要和当前批原文；每批同时返回当前批译文与待用户审核的本批摘要。
- 严格校验每批翻译结果与文本块的对应关系，每批完整成功后写回对应 `TextData.translation`，并继续复用现有修补和渲染流程；后续批次失败不回滚已完成批次。
- 完成后保留项目状态并返回原编辑器逐页精修，统一使用原编辑器的现有导出功能。
- 章节翻译仅支持对话式 API LLM Provider；不支持本地 GGUF 模型及 DeepL、Google Cloud Translation、Caiyun 等传统机器翻译 Provider。
- 保持现有单页工具栏、普通流水线、项目文件格式和导出接口兼容，不引入独立 Web 应用或新的章节数据模型。

## Capabilities

### New Capabilities

- `chapter-translation`: 提供 App 内整章配置、单次或摘要继承分批翻译、逐批结果写回、任务反馈和返回逐页精修的完整流程。

### Modified Capabilities


## Impact

- 前端：`MenuBar`、新的章节翻译路由与工作区组件、API schema 和多语言文案。
- 后端：新增章节翻译长任务入口与聚合服务，复用现有 `pipeline::run`、LLM Provider、场景节点和历史操作。
- LLM Provider：让已有 `LlmGenerationOptions.max_tokens` 对 OpenAI、OpenAI-compatible、Claude、Gemini、DeepSeek 等对话式 API Provider 生效。
- 兼容性：不修改 `.khr` 项目结构；现有编辑、渲染、导出和单页翻译行为保持不变。
