# Koharu Context

这是基于 Koharu 的章节上下文翻译 Fork。本 README 只介绍本 Fork 新增的功能，以及如何在 Windows 上从零启动开发环境并编译可直接运行的 `koharu.exe`。

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

### 处理配置与阅读体验

- 顶栏“配置”菜单可以保存、应用、更新和删除处理配置；配置包含流水线引擎、当前 API 模型、目标语言、阅读顺序、字体与渲染样式、章节翻译参数等。
- 全新安装内置“日漫”和“韩语”两套配置并默认选中“日漫”：日漫使用 Manga OCR，韩语配置使用 PaddleOCR-VL 1.6；已有用户配置不会被覆盖或重复追加。
- 处理配置不保存 API Key、数据目录或 HTTP 参数。Provider API Key 保存在当前数据路径下的 `secrets/provider-api-keys.toml`，该文件已被 Git 忽略。
- 章节翻译优先继承主页已经加载的 API LLM，进入章节页面后仍可单独切换模型。
- 翻译完成后可在主页使用键盘 `←` / `→` 快速切换上一张和下一张图片；在输入框、下拉框或弹窗中操作时不会误触翻页。

## 使用流程

1. 按漫画类型从顶栏“配置”菜单选择日漫、韩语或自己保存的配置。
2. 在原编辑器中导入一章漫画，并确认页面顺序。
3. 点击顶栏“章节翻译”。
4. 选择 API Provider、模型、目标语言，并按需填写作品背景与术语提示词。
5. 选择整章一次翻译，或开启分批并设置每批页数。
6. 开始任务，等待页面准备与 OCR 完成。
7. 若使用分批模式，每批完成后检查当前摘要，修改后点击“继续下一批”。
8. 全部完成后返回编辑器逐页阅读和精修。
9. 在原编辑器中导出最终图片。

## Windows 下载安装

普通用户无需安装 Git、Rust、Bun、Visual Studio 或 CUDA Toolkit。打开项目的 [GitHub Releases](https://github.com/Wkingxc/koharu-context/releases) 页面，下载最新版本的：

```text
Koharu_版本号_x64-setup.exe
```

双击安装器并按提示完成安装。由于当前安装器尚未进行商业代码签名，Windows SmartScreen 可能显示保护提示；确认文件来自本仓库后，可点击“更多信息”→“仍要运行”。

应用和界面可在安装后直接启动。模型与部分 GPU 运行库会在首次使用对应引擎时下载，因此首次处理漫画需要保持网络连接；没有兼容 NVIDIA GPU 时会自动回退到 CPU。后续版本目前需要从 Releases 页面手动下载安装，新版本会沿用原有数据目录和配置。

## Windows 从零编译运行（开发者）

以下步骤以 Windows 10/11 x64、PowerShell 为例。当前仓库的 Windows 桌面构建默认启用 CUDA，因此即使之后使用 `--cpu` 运行，编译阶段仍需要能找到 `nvcc`。

### 1. 安装 Git

从 [Git for Windows](https://git-scm.com/download/win) 安装 Git。安装后重新打开 PowerShell：

```powershell
git --version
```

### 2. 安装 Visual Studio C++ Build Tools

从 [Visual Studio Downloads](https://visualstudio.microsoft.com/downloads/) 下载 **Build Tools for Visual Studio**。在安装器中选择：

- **Desktop development with C++（使用 C++ 的桌面开发）**
- MSVC x64/x86 C++ build tools
- Windows 10 SDK 或 Windows 11 SDK

本仓库的构建脚本会通过 Visual Studio Installer 自带的 `vswhere` 查找 `cl.exe`，通常不需要手动修改 PATH。

### 3. 确认 WebView2

Tauri 桌面界面依赖 Microsoft Edge WebView2。大多数 Windows 10/11 系统已经预装；若启动程序时提示缺失，请安装 [WebView2 Evergreen Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)。

### 4. 安装 Rust 1.95 或更高版本

从 [Rust 官方安装页](https://rust-lang.org/tools/install/) 下载并运行 `rustup-init.exe`，使用默认的 MSVC toolchain。完成后重新打开 PowerShell：

```powershell
rustup default stable-msvc
rustup update stable
rustc --version
cargo --version
```

确保 `rustc` 版本不低于 `1.95`。

### 5. 安装 Bun

在 PowerShell 中执行 [Bun 官方安装命令](https://bun.sh/docs/installation)：

```powershell
powershell -c "irm bun.sh/install.ps1 | iex"
```

重新打开 PowerShell 后验证：

```powershell
bun --version
```

### 6. 安装 CUDA Toolkit 13.0

从 [NVIDIA CUDA Toolkit 13.0 下载页](https://developer.nvidia.com/cuda-13-0-0-download-archive) 安装 Windows x86_64 版本。建议使用默认安装目录：

```text
C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.0
```

重新打开 PowerShell 后验证：

```powershell
nvcc --version
```

若命令仍不可用，可在当前 PowerShell 临时设置：

```powershell
$env:CUDA_PATH = "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.0"
$env:Path = "$env:CUDA_PATH\bin;$env:Path"
nvcc --version
```

要使用 NVIDIA GPU 运行，还需要安装支持 CUDA 13 的较新 NVIDIA 驱动。没有可用 NVIDIA GPU 时仍可在编译后使用 `--cpu` 启动。

### 7. 拉取代码并安装依赖

```powershell
git clone https://github.com/Wkingxc/koharu-context.git
cd koharu-context
bun install
```

首次安装和编译会下载较多前端、Rust 与模型相关依赖，请保持网络连接。

### 8. 开发模式启动

```powershell
bun run dev
```

该命令会启动前端开发服务、Rust 后端和 Tauri 桌面窗口。首次 Rust 编译耗时较长属于正常现象。

### 9. 编译 Windows NSIS 安装器

```powershell
bun run build
```

该命令会先构建前端，再以 Release 模式编译 NSIS 安装器。成功后安装包位于：

```text
target\release\bundle\nsis\Koharu_版本号_x64-setup.exe
```

只需要裸 EXE 进行开发调试时执行：

```powershell
bun run build:binary
```

裸程序位于 `target\release\koharu.exe`，可以直接启动或强制使用 CPU：

```powershell
.\target\release\koharu.exe
.\target\release\koharu.exe --cpu
```

换到另一台电脑运行时，仍需满足 WebView2、Microsoft Visual C++ Runtime 等系统运行依赖；模型和部分运行库会在首次使用相关功能时下载。

### macOS 构建产物

在满足 Rust、Bun 和 Xcode Command Line Tools 的 macOS 上执行：

```bash
bun install
bun run build
```

默认只生成可由 Finder 直接启动的应用：

```text
target/release/bundle/macos/Koharu.app
```

本地产物未进行 Apple Developer ID 签名或公证；仅需裸二进制时可使用 `bun run build:binary`。

## Windows 常见问题

### `nvcc not found`

确认 CUDA Toolkit 已安装，重新打开 PowerShell，并运行 `nvcc --version`。构建脚本会优先读取 `CUDA_PATH`，其次扫描 CUDA 默认安装目录。

### `cl.exe not found`

重新打开 Visual Studio Installer，确认已安装“使用 C++ 的桌面开发”和 x64 MSVC 工具。无需单独安装完整 Visual Studio IDE，Build Tools 即可。

### `bun`、`rustc` 或 `cargo` 无法识别

关闭所有终端后重新打开 PowerShell。若仍失败，确认以下目录位于用户 PATH：

```text
%USERPROFILE%\.bun\bin
%USERPROFILE%\.cargo\bin
```

### 界面无法打开或白屏

安装或修复 WebView2 Evergreen Runtime，然后重新启动 `koharu.exe`。

### 中文渲染为空白

在 Windows 中安装可用的中文字体，并在编辑器中选择该字体。推荐 Noto Sans SC；不要选择当前系统不存在的字体。

### 想重新完整编译

先关闭正在运行的 Koharu，再执行：

```powershell
cargo clean
bun install
bun run build
```

`cargo clean` 会删除已有 Rust 编译产物，下一次构建会明显更慢，仅在缓存或构建产物异常时使用。
