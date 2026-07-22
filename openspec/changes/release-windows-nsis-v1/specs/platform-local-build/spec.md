## MODIFIED Requirements

### Requirement: Windows 默认产出 NSIS 安装器
项目 SHALL 在 Windows 执行 `bun run build` 时仅生成 NSIS 安装器，并 SHALL 通过 `bun run build:binary` 保留裸 `koharu.exe` 构建入口。

#### Scenario: Windows 默认构建
- **WHEN** 开发者在 Windows 执行 `bun run build`
- **THEN** 构建命令使用 `tauri build --bundles nsis`

#### Scenario: Windows 裸二进制构建
- **WHEN** 开发者执行 `bun run build:binary`
- **THEN** 构建命令使用 `tauri build --no-bundle`
