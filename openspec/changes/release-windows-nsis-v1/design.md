## Context

Tauri 已启用 bundle 和 Windows 图标，但 `scripts/build.ts` 在 Windows 传入 `--no-bundle`。现有 `.github/workflows/release.yml` 来自上游，包含 macOS/Linux、Winget、容器、Azure/Apple 签名以及 updater 产物；应用更新器端点也仍指向 `mayocream/koharu`。

## Goals / Non-Goals

**Goals:**
- 用户从 GitHub Releases 下载单个 NSIS 安装器即可安装 Windows x64 版本。
- 推送 `v1.0.0` 标签自动创建 Release 并上传安装器。
- 发布不依赖私有签名、Winget 或 updater 密钥。
- 保留开发者生成裸 EXE 的能力。

**Non-Goals:**
- 不生成 MSI、便携压缩包、macOS、Linux、Winget 或容器产物。
- 不实现自动更新。
- 不配置 Windows 代码签名证书。

## Decisions

- GitHub Actions 仅保留 `windows-2022` job，通过 `tauri-apps/tauri-action` 传入 `--bundles nsis` 并使用仓库自带 `GITHUB_TOKEN` 创建 Release。
- 标签使用带 `v` 前缀的 `v1.0.0`，Cargo/Tauri 内部版本使用合法 SemVer `1.0.0`。版本脚本分别处理显示标签和内部版本。
- Windows 本地 `bun run build` 传入 `--bundles nsis`；`bun run build:binary` 保持 `--no-bundle`。
- 移除 Tauri updater 后端注册和上游端点，并取消前端 `UpdaterProvider` 挂载，使设置页保留版本展示但不发起更新请求。

## Risks / Trade-offs

- [未签名安装器触发 SmartScreen] → README 明示“更多信息 → 仍要运行”；后续获得证书后可独立增加签名。
- [GitHub Windows runner 的 CUDA 构建耗时较长] → 沿用已验证的 CUDA Toolkit 13.0 action 与 MSVC 配置。
- [首次模型使用仍需下载] → 安装器只包含应用本体，README 说明首次使用需要网络。
