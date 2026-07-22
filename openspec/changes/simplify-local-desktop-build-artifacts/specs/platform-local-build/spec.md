## ADDED Requirements

### Requirement: 平台感知的直接运行产物
系统 SHALL 让 `bun run build` 在 macOS 只生成可直接启动的 `.app`，在 Windows 只生成可直接启动的 Release EXE，并 MUST 不默认生成 DMG 或 NSIS 安装器。

#### Scenario: macOS 本地构建
- **WHEN** 用户在 macOS 执行 `bun run build`
- **THEN** 系统生成 `target/release/bundle/macos/Koharu.app` 且不执行 DMG 打包

#### Scenario: Windows 本地构建
- **WHEN** 用户在 Windows 执行 `bun run build`
- **THEN** 系统生成 `target\release\koharu.exe` 且不执行 NSIS 打包

### Requirement: Windows 图标保持有效
系统 MUST 保留 EXE 资源和主窗口显式图标设置，使直接运行 Windows EXE 时任务栏显示 Koharu 图标。

#### Scenario: 直接运行 Windows EXE
- **WHEN** 用户双击新构建的 `target\release\koharu.exe`
- **THEN** Windows 任务栏显示 Koharu 应用图标
