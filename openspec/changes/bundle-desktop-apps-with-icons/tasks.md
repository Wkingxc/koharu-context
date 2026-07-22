## 1. 平台打包

- [x] 1.1 将默认构建改为 macOS `app,dmg` 与 Windows `nsis` 平台包，保留 `build:binary`，并移除本地构建对上游签名凭据的依赖 | risk=medium | evidence=non-test

## 2. Windows 图标

- [x] 2.1 主窗口显式应用默认 Koharu 图标，并通过 Rust 编译和打包配置验证图标资源 | risk=medium | evidence=non-test

## 3. 文档与验证

- [x] 3.1 更新 Windows 构建教程，并完成格式、测试、生产构建、macOS 应用打包和 OpenSpec 严格校验 | risk=medium | evidence=comprehensive
