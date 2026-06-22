# Meter OCR App - 电表图片 OCR 识别与自动分拣工具

## 项目概述

基于 **Tauri v2 + Rust + ONNX Runtime (ort)** 的跨平台离线桌面客户端。
用户可以输入照片文件夹路径和编号列表，程序会自动识别图片中的文本，
匹配到对应编号后分拣到同名文件夹，匹配不到的图片移入「模糊图片」文件夹。

## 功能特性

- **离线 OCR 识别**：使用 PP-OCRv4 英文识别模型，完全离线运行
- **自动分拣**：识别图片文本后，匹配编号列表自动分类到对应文件夹
- **模糊图片处理**：匹配不到编号的图片自动移入「模糊图片」文件夹
- **实时进度**：显示处理进度、匹配统计、运行日志
- **跨平台**：支持 Windows 和 macOS

## 技术栈

| 组件 | 技术 | 版本 |
|------|------|------|
| 桌面框架 | Tauri v2 | 2.11.3 |
| 后端语言 | Rust | 1.96 |
| OCR 引擎 | ONNX Runtime (ort) | 2.0.0-rc.9 |
| 图像处理 | image | 0.25 |
| 数组计算 | ndarray | 0.16 |
| 前端 | HTML + CSS + Vanilla JS | - |
| 构建工具 | Vite | 6.x |

## 项目结构

```
meter-ocr-app/
├── index.html              # 前端界面
├── package.json            # Node.js 配置
├── vite.config.js          # Vite 构建配置
├── .rapidocr_onnxruntime/
│   ├── models/
│   │   └── en_PP-OCRv4_rec_infer.onnx  # OCR 模型文件 (7.3MB)
│   ├── onnxruntime.dll                 # ONNX Runtime 动态库 (14MB)
│   └── onnxruntime_providers_shared.dll
├── temp/
│   ├── images/             # 测试照片
│   └── numbered_folders/   # 测试编号文件夹
├── src-tauri/
│   ├── Cargo.toml          # Rust 依赖配置
│   ├── tauri.conf.json     # Tauri 配置
│   ├── build.rs
│   ├── capabilities/
│   │   └── default.json    # 权限配置
│   ├── icons/              # 应用图标
│   └── src/
│       ├── main.rs         # Rust 后端核心逻辑
│       └── lib.rs          # 库入口
└── 项目说明.txt
```

## 快速开始

### 前置要求

- [Rust](https://www.rust-lang.org/) 1.77+
- [Node.js](https://nodejs.org/) 18+
- [Tauri CLI v2](https://v2.tauri.app/) 2.x
- Windows: 安装 [MSVC Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)

### 安装步骤

```bash
# 1. 克隆项目后，进入项目目录
cd meter-ocr-app

# 2. 安装前端依赖
npm install

# 3. 运行开发模式（自动启动 Vite + Tauri）
cargo-tauri dev

# 4. 或构建发布版本
cargo-tauri build
```

● PowerShell 会话中 PATH 没有包含 cargo 的 bin 目录。需要先设置环境变量：

  $env:PATH = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" +
  [System.Environment]::GetEnvironmentVariable("Path","User"); cargo-tauri build

  或者直接运行：

  bash -c 'export PATH="$HOME/.cargo/bin:$PATH"; cd "g:\meter-ocr-app"; cargo-tauri build'

### 模型下载

程序需要以下文件位于 `.rapidocr_onnxruntime/` 目录：

1. **OCR 模型** `.rapidocr_onnxruntime/models/en_PP-OCRv4_rec_infer.onnx`
   - 从 [RapidOCR Releases](https://github.com/RapidAI/RapidOCR/releases) 下载
   - 或使用 `python download_model.py` 自动下载

2. **ONNX Runtime DLL** `.rapidocr_onnxruntime/onnxruntime.dll`
   - 从 [ONNX Runtime Releases](https://github.com/microsoft/onnxruntime/releases/download/v1.24.1/onnxruntime-win-x64-1.24.1.zip) 下载
   - 解压后从 `lib/` 目录复制 `onnxruntime.dll` 和 `onnxruntime_providers_shared.dll`

## 使用指南

1. 启动程序后，点击「浏览」选择包含照片的文件夹
2. 输入编号列表（每行一个编号），或点击「从已有文件夹加载编号」自动读取
3. 可选：指定输出目录（默认为图片目录下的 `output` 文件夹）
4. 点击「开始识别与分拣」
5. 程序会逐张识别图片中的文本，匹配到编号后自动复制到对应文件夹
6. 匹配不到的图片会移入「模糊图片」文件夹

## 核心算法

### 图像预处理
1. 读取图片并转换为 RGB
2. 等比缩放到高度 48px，宽度按比例（最大 480px）
3. 不足 480px 宽度的用黑色填充
4. 转换为 CHW 格式的 float32 数组，归一化到 [0, 1]

### 文本解码 (CTC Greedy)
模型输出为 `[1, T, C]` 的概率矩阵，使用 CTC 贪心解码：
- 每步取概率最高的字符索引
- 跳过 blank token (index 0)
- 去除连续重复字符

### 编号匹配
1. 移除 OCR 文本中的所有空白字符
2. 直接子串匹配：检查编号是否在识别文本中
3. 数字序列匹配：用正则 `\d{6,}` 提取连续数字，与编号中的数字部分交叉匹配

## 性能

- 单张图片 OCR 识别时间：~50-200ms（取决于硬件）
- 支持多线程推理（intra_threads=4）
- 8 张测试图片总处理时间：< 5 秒

## GitHub Actions 自动编译

已配置 `.github/workflows/build.yml`，支持一键编译 Windows / macOS (ARM64 + x64)。

### 使用方式

1. 在 GitHub 创建仓库并推送代码
2. 创建新 Tag 并推送（如 `v0.1.0`）
3. GitHub Actions 会自动触发编译

```bash
git tag v0.1.0
git push origin v0.1.0
```

4. 编译完成后在 Actions 页面下载产物

### 手动触发

在 GitHub Actions 页面选择 "Build" 工作流 → "Run workflow"

### 产物说明

| 平台 | 产物 |
|------|------|
| Windows | `.msi` 安装包 |
| macOS ARM64 | `.dmg` (Apple Silicon) |
| macOS x64 | `.dmg` (Intel) |

## License

本项目仅供学习和内部使用。
