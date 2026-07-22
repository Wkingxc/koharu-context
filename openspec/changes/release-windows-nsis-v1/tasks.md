## 1. NSIS 构建与发布

- [x] 1.1 将 Windows 本地默认构建切换为 NSIS，同时保留裸 EXE 构建入口 | risk=medium | evidence=non-test
- [x] 1.2 将 GitHub Release 工作流精简为 Windows x64 NSIS，并移除多平台、MSI、Winget、容器、签名和 updater 产物 | risk=high | evidence=non-test

## 2. Fork 发布边界

- [x] 2.1 禁用原版 updater 连接并将应用内仓库入口切换到 Fork | risk=medium | evidence=non-test
- [x] 2.2 更新版本脚本与 README，支持内部版本 `1.0.0`、标签 `v1.0.0` 和下载安装说明 | risk=medium | evidence=non-test

## 3. 验证与发布

- [x] 3.1 完成 UI/Rust 测试、生产构建、工作流与配置校验及 OpenSpec 严格校验 | risk=high | evidence=comprehensive
- [ ] 3.2 提交并推送发布配置，创建发布提交和 `v1.0.0` 标签并推送以触发 GitHub Release | risk=high | evidence=external
