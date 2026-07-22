## ADDED Requirements

### Requirement: 全新安装提供两套漫画处理配置
系统 SHALL 在不存在处理配置持久化状态时提供“日漫”和“韩语”两套内置配置，并 MUST 默认选中“日漫”。

#### Scenario: 首次启动
- **WHEN** 用户首次启动应用且不存在 `koharu-processing-profiles` 持久化状态
- **THEN** 配置菜单按“日漫”、“韩语”的顺序展示两套配置并将“日漫”标记为当前配置

#### Scenario: 应用日漫配置
- **WHEN** 用户选择“日漫”配置
- **THEN** 系统应用已固化的 Manga OCR、漫画流水线、LLM、字体、阅读顺序和章节翻译参数

#### Scenario: 应用韩语配置
- **WHEN** 用户选择“韩语”配置
- **THEN** 系统应用已固化的 PaddleOCR-VL、漫画流水线、LLM、字体、阅读顺序和章节翻译参数

### Requirement: 保留已有用户配置
系统 MUST NOT 使用内置配置覆盖或重复追加已有的持久化处理配置。

#### Scenario: 已有持久化配置
- **WHEN** 应用启动时存在 `koharu-processing-profiles` 状态
- **THEN** 系统恢复该状态中的配置和当前配置选择，而不注入内置配置
