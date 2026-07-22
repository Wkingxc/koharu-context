## ADDED Requirements

### Requirement: 仅发布 Windows NSIS 安装器
系统 SHALL 在推送版本标签时只构建 Windows x64 NSIS 安装器，并 MUST 将该安装器上传到对应 GitHub Release。

#### Scenario: 推送 v1.0.0 标签
- **WHEN** 维护者将 `v1.0.0` 标签推送到 GitHub
- **THEN** GitHub Actions 在 Windows runner 上构建版本 `1.0.0` 并创建名为 `v1.0.0` 的 Release

#### Scenario: Release 产物
- **WHEN** 发布工作流成功完成
- **THEN** Release 包含一个 NSIS `x64-setup.exe`，且不包含 MSI、Winget、容器、macOS 或 Linux 构建产物

### Requirement: 本地构建区分安装器与裸二进制
系统 SHALL 让 Windows 的默认构建命令生成 NSIS 安装器，并 SHALL 保留单独生成裸 EXE 的命令。

#### Scenario: 默认 Windows 构建
- **WHEN** 开发者在 Windows 执行 `bun run build`
- **THEN** Tauri 使用 `--bundles nsis` 并在 `target/release/bundle/nsis` 生成安装器

#### Scenario: 裸二进制构建
- **WHEN** 开发者执行 `bun run build:binary`
- **THEN** Tauri 使用 `--no-bundle` 仅生成 `target/release/koharu.exe`

### Requirement: Fork 发布不连接原版更新端点
系统 MUST NOT 在 Fork 应用启动或打开设置时向 `mayocream/koharu` 的 updater 端点检查更新。

#### Scenario: 启动 v1.0.0
- **WHEN** 用户启动 Fork 的安装版本或打开设置页
- **THEN** 应用不加载原版 updater provider 且不请求原版 Release 更新元数据
