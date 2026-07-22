## 1. 章节准备错误恢复

- [x] 1.1 展示缺失 OCR 的页码、页面名称和文本块，并提供返回编辑器修复后重试的路径 | risk=medium | evidence=focused

## 2. 漫画处理配置预设

- [x] 2.1 实现预设的本地持久化、保存、应用和删除，并覆盖引擎、LLM、语言、字体及章节翻译参数 | risk=high | evidence=focused
- [x] 2.2 在中英文顶部菜单中提供预设切换和管理交互 | risk=medium | evidence=focused

## 3. 本地 Provider 密钥

- [x] 3.1 将 API Key 持久化从系统钥匙串替换为数据目录内的 Git 忽略文件，并保持配置脱敏和安全权限 | risk=high | evidence=focused

## 4. 回归验证

- [x] 4.1 验证 Rust 与 UI 测试、格式、类型、生产构建和 OpenSpec 严格校验 | risk=high | evidence=comprehensive
