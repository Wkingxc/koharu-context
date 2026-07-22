## 1. 当前配置更新

- [x] 1.1 实现仅覆盖当前选中配置的 store 行为和顶部菜单中英文交互，并以聚焦测试验证身份、顺序及其他配置保持不变 | risk=medium | evidence=focused

## 2. 默认引擎组合

- [x] 2.1 将 `PipelineConfig::default()` 更新为指定八阶段引擎并验证全新默认及已有配置保留行为 | risk=medium | evidence=focused

## 3. 变更验证

- [x] 3.1 完成相关 UI、Rust、构建和 OpenSpec 严格校验 | risk=medium | evidence=comprehensive
