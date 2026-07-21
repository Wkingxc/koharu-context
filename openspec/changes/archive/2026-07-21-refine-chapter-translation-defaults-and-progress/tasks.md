## 1. 配置默认值

- [x] 1.1 将章节翻译默认最大输出 Token 调整为 `32000`，并用现有配置页测试验证默认请求值 | risk=low | evidence=focused

## 2. 图片数量进度

- [x] 2.1 扩展章节进度事件并按准备、翻译、后处理分别累计真实图片数量，验证缓存复用、批次边界和逐页后处理计数 | risk=high | evidence=focused
- [x] 2.2 将准备和后处理卡片改为展示 `X / N`，翻译卡片展示等待或批次状态，验证等待 LLM 时不伪造批内进度 | risk=medium | evidence=focused

## 3. 统一译文颜色

- [x] 3.1 章节译文写回时仅强制文字颜色为不透明黑色并保留其他显式样式，验证后续渲染不再使用预测文字颜色 | risk=medium | evidence=focused
