# Meter OCR — 图片 OCR 识别与自动分拣

基于 **Tauri v2 + Rust + ocrs/rten** 的跨平台离线桌面工具。

输入图片文件夹和编号列表，自动 OCR 识别 → 按编号后 N 位匹配 → 分拣到对应文件夹。

## 界面预览

![主界面](./screenshots/main.png)

## 功能

- **离线 OCR**：PP-OCR 模型（ocrs + rten），纯 Rust，无需网络
- **编号后 N 位匹配**：可配置匹配位数，默认后 7 位
- **递归扫描**：支持含子文件夹的图片目录
- **实时进度**：逐张显示识别文本和匹配结果
- **跨平台**：Windows + macOS (Apple Silicon)

## 技术栈

| 组件 | 技术 |
|------|------|
| 桌面框架 | Tauri v2 |
| 后端 | Rust |
| OCR | ocrs + rten (PP-OCR) |
| 前端 | HTML + CSS + JS (Vite 6) |

## 项目结构

```
meter-ocr-app/
├── index.html               # 前端界面
├── package.json              # Node.js 配置
├── vite.config.js            # Vite 配置
├── public/qrcode/            # 收款码图片
├── screenshots/              # README 配图
├── src-tauri/
│   ├── Cargo.toml            # Rust 依赖
│   ├── tauri.conf.json       # Tauri 配置
│   ├── models/               # OCR 模型 (.rten)
│   ├── capabilities/         # 权限配置
│   └── src/
│       ├── main.rs           # 入口
│       └── lib.rs            # 核心：OCR + 分拣
└── .github/workflows/
    └── build.yml             # CI 自动编译
```

## 快速开始

### 前置要求

- [Rust](https://www.rust-lang.org/) 1.77+
- [Node.js](https://nodejs.org/) 18+
- Windows: [MSVC Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)

### 安装运行

```bash
# 安装依赖
npm install

# 获取 OCR 模型（仅需一次）
cargo install ocrs-cli --locked
ocrs test.png                          # 自动下载模型
cp ~/.cache/ocrs/*.rten src-tauri/models/

# 开发模式
cargo tauri dev

# 构建
cargo tauri build

# PowerShell 如有 PATH 问题：
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
```

### 模型文件

| 文件 | 大小 |
|------|------|
| `text-detection.rten` | 2.4 MB |
| `text-recognition.rten` | 9.3 MB |

模型通过 `ocrs-cli` 首次运行时自动下载到 `~/.cache/ocrs/`，复制到 `src-tauri/models/` 后即可构建。

## 使用指南

1. 选择图片文件夹（可勾选「含子文件夹」递归扫描）
2. 输入编号列表，或从输出目录加载已有文件夹名
3. 设置匹配特征位数（编号后几位，默认 7）
4. 点击「开始识别与分拣」
5. 匹配成功 → 复制到对应编号文件夹
6. 未匹配 → 进入「模糊或没对应编码图片」

## CI / 自动编译

推送 tag 自动构建各平台包：

```bash
git tag v0.3.0 && git push origin v0.3.0
```

| 文件 | 平台 | 说明 |
|------|------|------|
| `setup.exe` | Windows x64 | 安装包 |
| `Portable.zip` | Windows x64 | 解压即用 |
| `.dmg` | macOS ARM64 | 拖入 Applications |

> Mac 首次打开会提示"无法验证开发者"，在「系统设置 → 隐私与安全性」中点击「仍要打开」。

## 打赏

如果这个工具对你有帮助：

| 微信 | 支付宝 |
|---|---|
| ![微信](./screenshots/qrcode-wechat.jpg) | ![支付宝](./screenshots/qrcode-alipay.jpg) |

## License

MIT
