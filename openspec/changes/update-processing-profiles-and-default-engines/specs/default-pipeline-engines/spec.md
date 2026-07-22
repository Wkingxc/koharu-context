## ADDED Requirements

### Requirement: 默认漫画处理引擎组合
系统 SHALL 在全新安装或流水线字段缺失时选择 `comic-text-bubble-detector`、`yuzumarker-font-detection`、`comic-text-detector-seg`、`speech-bubble-segmentation`、`manga-ocr`、`llm`、`aot-inpainting` 和 `koharu-renderer`，分别用于检测、字体检测、分割、对话框分割、OCR、翻译、修补和渲染。

#### Scenario: 全新安装
- **WHEN** 应用首次启动且不存在已持久化的流水线配置
- **THEN** 八个流水线阶段使用指定默认引擎

#### Scenario: 已有安装
- **WHEN** 应用加载包含完整流水线选择的已有配置
- **THEN** 系统保留用户已持久化的引擎选择而不以新默认值覆盖
