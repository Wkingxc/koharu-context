## ADDED Requirements

### Requirement: 默认生成平台应用包
系统 SHALL 让根目录 `bun run build` 构建当前平台的正式桌面应用包：macOS MUST 生成 `.app` 与 `.dmg`，Windows MUST 生成 NSIS 安装器。

#### Scenario: macOS 默认构建
- **WHEN** 用户在满足构建环境的 macOS 上执行 `bun run build`
- **THEN** 系统在 Tauri bundle 输出目录生成可由 Finder 直接启动的 `.app` 和可分发的 `.dmg`

#### Scenario: Windows 默认构建
- **WHEN** 用户在满足构建环境的 Windows 上执行 `bun run build`
- **THEN** 系统生成带应用资源的 NSIS 安装器，并保留编译得到的 Release EXE

### Requirement: 可选裸二进制构建
系统 SHALL 提供独立命令 `bun run build:binary`，以便用户明确跳过应用打包。

#### Scenario: 只构建二进制
- **WHEN** 用户执行 `bun run build:binary`
- **THEN** 系统使用 Tauri `--no-bundle` 构建 Release 裸二进制

### Requirement: Windows 显示应用图标
系统 SHALL 将配置中的 Koharu 应用图标显式设置到主窗口，并 MUST 将 `icon.ico` 用于 Windows 打包产物，使任务栏和安装后的应用入口显示应用图标。

#### Scenario: Windows 启动已打包应用
- **WHEN** 用户安装并启动 Windows 构建产物
- **THEN** 主窗口任务栏图标显示 Koharu 应用图标

### Requirement: 本地打包不依赖上游签名凭据
系统 MUST 不调用上游企业签名命令或要求更新包私钥完成普通本地构建。

#### Scenario: 无签名凭据构建
- **WHEN** 开发者未配置上游企业签名账号和 Tauri 更新签名私钥
- **THEN** 普通 `bun run build` 仍可执行平台应用打包
