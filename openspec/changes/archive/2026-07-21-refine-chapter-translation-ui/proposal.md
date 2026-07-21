## Why

章节翻译页当前把模型、提示词、分批设置和操作按钮集中在一个卡片中，视觉层级和纵向留白不足；模型数量较多时也缺少快速回到常用模型的方式。需要在不改变任务流程和后端协议的前提下，提高配置效率与可读性。

## What Changes

- 将翻译设置、分批设置和任务操作拆分为清晰的页面层级，并统一字段高度、卡片内边距与响应式间距。
- 在模型选择列表中提供收藏/取消收藏操作，收藏模型在当前 Provider 的匹配结果中优先展示。
- 将模型收藏作为用户偏好持久保存，重新打开应用后继续生效。

## Capabilities

### New Capabilities
- `chapter-translation-ui-refinement`: 覆盖章节翻译配置页的布局层级、响应式留白和模型收藏体验。

### Modified Capabilities

## Impact

影响 `ui/app/(app)/chapter-translation/page.tsx`、共享 `LlmModelSelect`、`preferencesStore`、章节翻译多语言文案及相关组件测试；不修改章节翻译 API、任务调度或渲染逻辑。
