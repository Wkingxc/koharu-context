## Why

当前 Fork 的 Windows 构建只生成裸 `koharu.exe`，普通用户必须自行搭建编译环境。原版发布工作流同时构建多个平台、Winget、容器和更新器产物，并依赖原作者的签名与发布配置，不适合本 Fork 的首次正式发布。

## What Changes

- 将 Fork 的正式版本提升为 `1.0.0`，Git 标签使用 `v1.0.0`。
- GitHub Release 只在 Windows x64 环境构建并上传 NSIS `setup.exe`。
- 本地 Windows `bun run build` 默认生成 NSIS 安装器，`build:binary` 继续生成裸 EXE。
- 移除原版多平台、MSI、Winget、容器、签名和更新器发布流程。
- 禁用客户端对原版仓库更新端点的连接，并将仓库入口指向本 Fork。
- README 优先提供下载安装说明，保留源码编译作为开发者文档。

## Capabilities

### New Capabilities
- `windows-nsis-release`: Windows NSIS 安装器的本地构建和 GitHub Release 自动发布。

### Modified Capabilities
- `platform-local-build`: Windows 默认构建产物由裸 EXE 调整为 NSIS 安装器，裸 EXE 改由专用脚本生成。

## Impact

影响 Tauri 打包配置、构建脚本、GitHub Release 工作流、版本脚本、更新器注册、仓库链接和 README。推送 `v1.0.0` 标签会在 GitHub 上创建真实 Release。
