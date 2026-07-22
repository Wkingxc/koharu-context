## Why

大章节中单个空 OCR 文本块会让准备阶段整体失败，但现有错误只显示数量，用户无法定位和修复；同时漫画类型切换需要反复调整引擎、模型和翻译参数，API Key 依赖系统钥匙串还会在 macOS 开发构建中频繁触发授权弹窗。

## What Changes

- 章节准备失败时展示缺失 OCR 的具体页码、页面名称和文本块，并提供返回编辑器修复后重试的路径。
- 新增可命名的漫画处理配置预设，保存处理引擎、LLM、语言、字体和章节翻译参数，并在顶部菜单栏快速切换。
- 将 Provider API Key 从操作系统钥匙串改为应用数据目录内的本地私密文件，配置与日志继续只暴露脱敏值。
- 配置预设不包含 API Key、数据路径、HTTP、外观和快捷键等机器级或全局设置。

## Capabilities

### New Capabilities

- `processing-config-profiles`: 保存、切换和删除漫画处理配置预设。
- `local-provider-secrets`: 在应用数据目录的 Git 忽略文件中持久化 Provider API Key。

### Modified Capabilities

- `chapter-translation`: 章节准备失败时必须提供可操作的 OCR 缺失定位信息和恢复入口。

## Impact

影响 `koharu-app` 配置与密钥持久化、章节翻译 RPC 错误信息、前端状态存储、顶部菜单栏、章节翻译失败页、中英文文案及相关测试。不会改变 `DataConfig.path` 的保存机制，也不会把 API Key 纳入配置预设或版本控制。
