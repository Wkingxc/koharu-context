## Context

Tauri 的单一 `build` 命令需要通过 CLI 参数决定是否打包。macOS 需要 `--bundles app`，而 Windows 本机测试只需要 `--no-bundle`。跨平台 `package.json` 不能可靠依赖 Bash 或 PowerShell 条件语法。

## Goals / Non-Goals

**Goals:**
- 一个 `bun run build` 命令在 macOS 与 Windows 生成各自最小的直接运行产物。
- 避免默认执行 DMG 和 NSIS 打包。

**Non-Goals:**
- 不提供面向发布分发的安装器、签名或公证流程。

## Decisions

- 新增 Bun 构建入口脚本读取 `os.type()`：Darwin 调用 `tauri build --bundles app`，Windows 调用 `tauri build --no-bundle`，其他平台暂时沿用 `--no-bundle`。相比 shell 条件表达式，该方案能在 PowerShell 与 macOS shell 中保持一致。
- 平台 Tauri 配置同步移除 DMG 与 NSIS 目标，避免直接调用 Tauri CLI 时产生与默认脚本相反的结果。

## Risks / Trade-offs

- [没有安装器不便于对外分发] → 当前目标是本机运行测试；未来发布时可新增单独的发行构建命令。
