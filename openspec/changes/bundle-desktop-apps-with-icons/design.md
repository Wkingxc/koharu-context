## Context

Tauri 已配置有效的 `icon.icns`、`icon.ico` 和 PNG 图标，但根脚本使用 `tauri build --no-bundle`，绕过了平台应用包生成。通用配置中的 `trusted-signing-cli` 指向上游组织账号，`createUpdaterArtifacts` 也会引入本地开发者没有的更新签名私钥要求。主窗口由 Rust 在运行时通过 `WebviewWindowBuilder::from_config` 创建。

## Goals / Non-Goals

**Goals:**
- 默认命令生成符合当前操作系统习惯的可安装或可直接启动应用包。
- 本地无签名凭据时也能打包。
- Windows 主窗口和任务栏可靠显示 Koharu 图标。

**Non-Goals:**
- 不在本次变更中配置 Apple 公证、Developer ID 或 Windows 代码签名证书。
- 不改变 CI 发布签名流程或自动更新服务。

## Decisions

- 根 `build` 命令移除 `--no-bundle`，平台配置分别限定 macOS 的 `app,dmg` 和 Windows 的 `nsis`，避免 `targets: all` 安装不必要的多套打包工具。
- 保留 `build:binary` 对应旧行为，便于开发者快速得到裸二进制。
- 普通本地构建关闭 `createUpdaterArtifacts` 并删除上游 `signCommand`。相比伪造或硬编码凭据，这是 Fork 可复现构建的安全做法。
- 在创建主窗口时从 `AppHandle::default_window_icon()` 克隆图标并显式调用 builder 的 `icon()`。相比只依赖 EXE 资源，这同时覆盖运行时窗口和任务栏图标。

## Risks / Trade-offs

- [未签名应用会触发系统安全提示] → README 明确本地产物未签名；正式发布时另行配置自身证书和公证。
- [无法在 macOS 实机验证 Windows 任务栏] → 通过 Tauri 配置校验、Windows 专用图标资源和编译期 API 校验覆盖，最终任务栏外观需在 Windows 构建后确认。
