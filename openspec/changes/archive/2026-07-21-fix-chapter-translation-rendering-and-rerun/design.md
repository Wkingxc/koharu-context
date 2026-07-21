## Context

编辑器的普通渲染请求从 `usePreferencesStore.defaultFont` 传入 `PipelineRunOptions.default_font`，而章节翻译内部调用 `run_pipeline` 时固定使用 `PipelineRunOptions::default()`。因此没有局部字体的中文文本会落到脚本默认的 `PingFang SC`，在当前字体注册表不可用时整块文字渲染失败。章节完成页又通过持久的 `operationId` 判断完成态，返回编辑器时未重置该状态。

## Goals / Non-Goals

**Goals:**
- 让章节翻译与原编辑器使用同一个默认字体输入，并继续由现有渲染器处理局部样式优先级。
- 让完成页返回动作结束本次页面状态，支持再次进入和覆盖执行。
- 删除章节页重复导出入口。

**Non-Goals:**
- 不修改字体选择器、字体下载机制或渲染器的通用字体回退策略。
- 不增加章节翻译专用导出实现，也不改变原编辑器导出格式。

## Decisions

- 在 `StartChapterTranslationRequest` 增加可选 `defaultFont`，由章节页读取与普通渲染相同的 `usePreferencesStore.defaultFont`。相比后端猜测系统字体或读取未同步的项目元数据，这能准确复用用户发起时的编辑器设置。
- 章节翻译仅在修补渲染管线中传入 `default_font`；已有 `TextStyle.font_families` 仍由渲染器优先使用，因此不额外改写文本样式。
- 完成页“返回精修”在导航前调用 `resetRun`，保留失败批次的重试状态和运行中离开后再观察进度的现有行为。

## Risks / Trade-offs

- [用户选择的字体本身不可用时仍可能渲染失败] → 继续沿用原编辑器已有的字体可用性与下载机制，本次只消除章节流程丢失字体的差异。
- [重新执行会覆盖旧译文和渲染图] → 这是迭代流程的预期行为，所有写入继续通过现有历史操作完成。
