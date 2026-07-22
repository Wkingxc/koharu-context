## Why

本项目主要在 Windows 本机运行和测试，不需要每次生成安装器；macOS 也只需要 Finder 可直接启动的 `.app`，默认生成 DMG 与 NSIS 会增加无用耗时和概念负担。

## What Changes

- 默认 `bun run build` 根据当前平台选择最小可直接运行产物。
- macOS 只生成 `Koharu.app`，不生成 DMG。
- Windows 只生成可直接双击的 `target\release\koharu.exe`，不生成 NSIS 安装器。
- 保留 Windows EXE 的应用与任务栏图标修复。
- README 改为说明直接运行 EXE 和 macOS APP 的产物位置。

## Capabilities

### New Capabilities
- `platform-local-build`: 面向 macOS APP 与 Windows EXE 的平台感知本地构建流程。

### Modified Capabilities

## Impact

影响根构建脚本、Tauri 平台 bundle 目标与 README；不改变应用运行逻辑、模型或用户配置。
