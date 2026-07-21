# Koharu Context

这是基于 Koharu 的章节上下文翻译 Fork。本 README 只介绍本 Fork 新增的功能，以及如何在 Windows 上从零启动开发环境并编译出可直接运行的 `koharu.exe`。

## 本 Fork 的改动

### 章节级上下文翻译

- 在桌面端顶栏新增“章节翻译”独立页面。
- 先按项目页面顺序完成检测与 OCR，再把所有有效文本块组织成带页面和文本块 ID 的结构化输入。
- 没有文字的页面会保留原有页面位置，但不会生成无效翻译项，也不会影响后续页面对应、修补或渲染。
- 翻译结果按文本块 ID 写回原项目，返回编辑器后可继续逐页精修，最终仍使用原编辑器的导出功能。
- 再次进入章节翻译可以重新执行并覆盖旧译文和渲染结果，形成“章节翻译 → 精修 → 重译覆盖 → 精修 → 导出”的工作流。

### 整章与分批翻译

- 关闭分批时，整章文本一次请求给所选 API 模型。
- 开启分批时，可按页数设置每批大小。
- 每批翻译完成后，模型同时返回本批摘要；程序暂停并展示摘要，用户可以校对和修改。
- 只有用户点击“继续下一批”后才会发送下一批请求。
- 后续批次会携带全部已确认摘要；历史摘要只读、可折叠，当前批次摘要可编辑。
- 不传递不断膨胀的完整对话历史，翻译正文仍保持简单、可校验的 JSON 结构。

### API 模型与提示词

- 章节翻译只使用 API 模型，支持 OpenAI、OpenAI-compatible、Claude、Gemini 和 DeepSeek provider。
- 用户可选择模型、收藏常用模型、设置目标语言和最大输出 Token；默认最大输出为 `32000`。
- 支持输入作品背景、固定译名、称谓和语气等中文提示词。
- 模型必须返回文本块 ID 与译文；分批模式额外返回本批摘要。程序会校验 ID 完整性、重复项和空译文后再写回。

### 进度、渲染与精修衔接

- 准备阶段和修补渲染阶段按真实页面数量显示 `已完成页数 / 总页数`。
- LLM 请求按整章或批次显示状态，不伪造成逐页翻译进度。
- 章节翻译继承编辑器中的全局字体与文本块局部样式，初次写回的译文字色统一为黑色。
- 翻译完成后直接回到原编辑器精修；章节页不再重复提供“导出全部”。
- 界面文案只维护简体中文和英文。

## 使用流程

1. 在原编辑器中导入一章漫画，并确认页面顺序。
2. 点击顶栏“章节翻译”。
3. 选择 API Provider、模型、目标语言，并按需填写作品背景与术语提示词。
4. 选择整章一次翻译，或开启分批并设置每批页数。
5. 开始任务，等待页面准备与 OCR 完成。
6. 若使用分批模式，每批完成后检查当前摘要，修改后点击“继续下一批”。
7. 全部完成后返回编辑器逐页精修。
8. 在原编辑器中导出最终图片。

## Windows 仓库级构建方案

以下步骤以 Windows 10/11 x64、PowerShell 为例。目标是让克隆仓库后的工具链、依赖缓存、模型和应用数据尽量只写入仓库目录，不修改用户级 PATH。

### 目录规划

| 内容 | 仓库内位置 |
| --- | --- |
| Rustup 工具链 | `.tools/rustup` |
| Cargo 与 Rust 依赖缓存 | `.tools/cargo` |
| Bun 可执行文件 | `.tools/bun` |
| Bun 包缓存 | `.cache/bun` |
| Micromamba 与包缓存 | `.tools/micromamba`、`.tools/mamba-root` |
| CUDA Toolkit | `.tools/cuda` |
| 前端依赖 | `node_modules`、`ui/node_modules` |
| Rust 编译产物 | `target` |
| Koharu 配置、模型、运行库和项目 | `.koharu-data` |

`.tools`、`.cache`、`.koharu-data`、`node_modules` 和 `target` 均被 Git 忽略，不会提交大体积工具或用户数据。

### 无法完全放入仓库的系统组件

以下组件必须或可能写入 Windows 系统位置：

- NVIDIA 显卡驱动：GPU 运行所需的内核驱动，无法做成项目级依赖。
- Microsoft Edge WebView2 Runtime：Tauri 桌面窗口的系统运行时。
- Visual Studio Installer、MSVC 和 Windows SDK：Build Tools 主体可以指定仓库内安装路径，但安装器元数据、注册信息和部分 Windows SDK 仍可能写入系统目录。
- API 密钥：应用继续使用 Windows Credential Manager 安全保存，不写入仓库明文文件。

除此之外，克隆仓库后的下载均按下面步骤限制在仓库目录。

### 固定的工具版本

本教程固定以下已验证版本，避免每次安装自动漂移：

| 工具 | 版本 |
| --- | --- |
| Rust | `1.97.1-x86_64-pc-windows-msvc` |
| Bun | `1.3.14` |
| Micromamba | `2.8.1-0` |
| CUDA Toolkit | `13.0.0` |

JavaScript 和 Rust 包的精确版本分别由仓库中的 `bun.lock` 与 `Cargo.lock` 固定。

### 1. 准备源码与系统组件

使用已有 Git，或从 GitHub 下载源码 ZIP：

```powershell
git clone https://github.com/Wkingxc/koharu-context.git
cd koharu-context
```

从 [Visual Studio Downloads](https://visualstudio.microsoft.com/downloads/) 下载 **Build Tools for Visual Studio**，选择 **Desktop development with C++（使用 C++ 的桌面开发）**、x64 MSVC 和 Windows 10/11 SDK。

如果希望 Build Tools 主体也位于仓库，可在 Visual Studio Installer 中把安装位置设置为：

```text
<仓库>\.tools\vs-buildtools
```

构建脚本会通过 `vswhere` 自动查找该安装实例的 `cl.exe`。

确认系统已安装 [WebView2 Evergreen Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)。使用 NVIDIA GPU 时还需要单独安装较新的 NVIDIA 驱动；不使用 GPU 时，编译完成后可以通过 `--cpu` 启动。

### 2. 初始化当前 PowerShell 的仓库级环境

在仓库根目录执行。环境变量只对当前 PowerShell 生效，不会修改系统或用户环境变量：

```powershell
$Repo = (Resolve-Path .).Path
$Tools = Join-Path $Repo ".tools"

$env:RUSTUP_HOME = Join-Path $Tools "rustup"
$env:CARGO_HOME = Join-Path $Tools "cargo"
$env:BUN_INSTALL = Join-Path $Tools "bun"
$env:BUN_INSTALL_CACHE_DIR = Join-Path $Repo ".cache\bun"
$env:MAMBA_ROOT_PREFIX = Join-Path $Tools "mamba-root"
$env:KOHARU_DATA_ROOT = Join-Path $Repo ".koharu-data"

New-Item -ItemType Directory -Force `
  $env:RUSTUP_HOME, `
  $env:CARGO_HOME, `
  $env:BUN_INSTALL, `
  $env:BUN_INSTALL_CACHE_DIR, `
  $env:MAMBA_ROOT_PREFIX, `
  $env:KOHARU_DATA_ROOT, `
  (Join-Path $Tools "downloads"), `
  (Join-Path $Tools "micromamba") | Out-Null

$env:Path = "$env:CARGO_HOME\bin;$env:BUN_INSTALL\bin;$env:Path"
```

每次打开新的 PowerShell，都需要重新执行本节环境变量设置，确保不会回退到用户目录中的其他 Rust、Bun 或应用数据。

### 3. 将 Rust 安装到仓库

```powershell
$RustupInit = Join-Path $Tools "downloads\rustup-init.exe"
Invoke-WebRequest "https://win.rustup.rs/x86_64" -OutFile $RustupInit

& $RustupInit `
  -y `
  --no-modify-path `
  --profile minimal `
  --default-host x86_64-pc-windows-msvc `
  --default-toolchain 1.97.1

rustc --version
cargo --version
```

`RUSTUP_HOME` 保存工具链，`CARGO_HOME` 保存 Cargo 可执行文件、registry 和 git 依赖缓存，两者都位于 `.tools`。

### 4. 将 Bun 安装到仓库

使用 Bun 官方安装脚本的固定版本和无系统修改参数：

```powershell
iex "& {$(irm https://bun.sh/install.ps1)} -Version 1.3.14 -NoPathUpdate -NoRegisterInstallation -NoCompletions"

bun --version
```

`BUN_INSTALL` 控制 Bun 可执行文件位置，`BUN_INSTALL_CACHE_DIR` 控制依赖下载缓存位置。上述命令不会修改用户 PATH、注册已安装程序或写入 PowerShell 补全配置。

### 5. 将 Micromamba 和 CUDA 安装到仓库

Micromamba 是单文件工具，`MAMBA_ROOT_PREFIX` 会把环境和下载缓存约束在仓库内：

```powershell
$MambaDir = Join-Path $Tools "micromamba"
$Mamba = Join-Path $MambaDir "micromamba.exe"

Invoke-WebRequest `
  "https://github.com/mamba-org/micromamba-releases/releases/download/2.8.1-0/micromamba-win-64.exe" `
  -OutFile $Mamba

& $Mamba create `
  --yes `
  --prefix (Join-Path $Tools "cuda") `
  --channel "nvidia/label/cuda-13.0.0" `
  "cuda=13.0.0"
```

激活仓库内 CUDA 环境：

```powershell
& $Mamba shell hook -s powershell | Out-String | Invoke-Expression
micromamba activate (Join-Path $Tools "cuda")

$env:CUDA_PATH = $env:CONDA_PREFIX
$env:Path = "$env:CUDA_PATH\bin;$env:CUDA_PATH\Library\bin;$env:Path"

nvcc --version
```

当前 Windows 桌面构建默认启用 CUDA，因此即使之后通过 `--cpu` 运行，编译阶段仍需要 `nvcc`。仓库已有的 `scripts/dev.ts` 会优先使用这里设置的 `CUDA_PATH`。

### 6. 安装锁定的项目依赖

```powershell
bun install --frozen-lockfile
```

前端包会安装到仓库的 `node_modules`，下载缓存写入 `.cache/bun`。Rust 依赖会在首次编译时按 `Cargo.lock` 下载到 `.tools/cargo`。

不要执行 `bun update`、`cargo update` 或使用不带版本号的工具升级命令，否则会主动改变锁定版本。

### 7. 开发模式启动

确认当前 PowerShell 已完成第 2 节环境变量设置和第 5 节 CUDA 激活，然后执行：

```powershell
bun run dev
```

因为设置了 `KOHARU_DATA_ROOT`，程序首次启动后下载的模型、运行库、字体缓存、配置和项目数据都会写入：

```text
<仓库>\.koharu-data
```

### 8. 编译 Release EXE

```powershell
bun run build
```

该命令会构建前端并以 Release 模式编译 Tauri 应用，同时跳过安装包签名与打包。生成文件：

```text
target\release\koharu.exe
```

直接启动并继续使用仓库级数据目录：

```powershell
$env:KOHARU_DATA_ROOT = Join-Path $Repo ".koharu-data"
.\target\release\koharu.exe
```

强制使用 CPU：

```powershell
$env:KOHARU_DATA_ROOT = Join-Path $Repo ".koharu-data"
.\target\release\koharu.exe --cpu
```

这里生成的是可直接执行的 EXE，不是安装器。复制到另一台电脑时，仍需满足 WebView2、Microsoft Visual C++ Runtime 和相应显卡驱动等系统运行依赖。

## Windows 常见问题

### 新 PowerShell 中命令失效

仓库级方案故意不修改用户 PATH。打开新 PowerShell 后，重新执行“初始化当前 PowerShell 的仓库级环境”和“激活仓库内 CUDA 环境”两节。

### `nvcc not found`

确认 Micromamba CUDA 环境已激活，并检查：

```powershell
$env:CUDA_PATH
Get-Command nvcc
nvcc --version
```

### `cl.exe not found`

在 Visual Studio Installer 中确认已安装“使用 C++ 的桌面开发”、x64 MSVC 和 Windows SDK。Build Tools 即可，不要求安装完整 Visual Studio IDE。

### 界面无法打开或白屏

安装或修复 WebView2 Evergreen Runtime，然后重新启动 `koharu.exe`。

### 中文渲染为空白

在 Windows 中安装可用的中文字体，并在编辑器中选择该字体。推荐 Noto Sans SC；不要选择当前系统不存在的字体。

### 清理仓库级依赖

关闭 Koharu 后，删除 `.tools`、`.cache`、`node_modules`、`ui/node_modules` 和 `target` 即可清除下载与编译缓存。删除 `.koharu-data` 会同时删除模型、配置和项目数据，请先备份需要保留的项目。
