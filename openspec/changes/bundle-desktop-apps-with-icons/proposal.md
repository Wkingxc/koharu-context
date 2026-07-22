## Why

默认 `bun run build` 使用 `--no-bundle`，macOS 只得到会通过终端启动的裸 `exec`，Windows 也只得到缺少可靠任务栏图标的裸 EXE；现有打包配置还依赖上游企业签名，无法用于 Fork 的本地构建。

## What Changes

- 默认 `bun run build` 改为生成当前平台的正式桌面应用包。
- macOS 生成 `.app` 与 `.dmg`，Windows 生成 NSIS 安装器，同时保留 Release 裸二进制作为构建中间产物。
- 增加 `build:binary` 命令，供只需要裸二进制时使用。
- 移除不可用的上游 Windows 企业签名命令，并关闭需要私钥的更新包产物生成。
- 主窗口显式使用 Tauri 打包图标，确保 Windows 任务栏显示 Koharu 图标。
- 更新 README 中 Windows 构建产物与启动说明。

## Capabilities

### New Capabilities
- `desktop-app-bundling`: macOS 与 Windows 的默认本地构建产物、备用裸二进制构建和桌面窗口图标行为。

### Modified Capabilities

## Impact

影响根目录 `package.json`、Tauri 通用及平台配置、主窗口创建代码和 README；不改变业务数据、模型目录或运行时配置。
