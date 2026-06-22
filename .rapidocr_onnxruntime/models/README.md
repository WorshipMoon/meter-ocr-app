# 模型下载说明

## PP-OCRv4 英文识别模型

本目录需要包含以下文件：

### 必需文件
- `en_PP-OCRv4_rec_infer.onnx` - PP-OCRv4 英文文本识别模型

### 下载方式

#### 方式一：从 GitHub Release 下载
访问 https://github.com/RapidAI/RapidOCR/releases 下载模型压缩包

#### 方式二：从 HuggingFace 下载
```bash
curl -L -o en_PP-OCRv4_rec_infer.onnx https://huggingface.co/RapidAI/PP-OCRv4/resolve/main/en_PP-OCRv4_rec_infer.onnx
```

#### 方式三：使用 RapidOCR 自动下载
```bash
# Python 环境
pip install rapidocr-onnxruntime
python -c "from rapidocr_onnxruntime import RapidOCR; RapidOCR()"
```

### 安装后目录结构
```
.rapidocr_onnxruntime/models/
└── en_PP-OCRv4_rec_infer.onnx   (~10MB)
```

## 字符集文件 (可选)

如果有 `ppocr_keys_v1.txt` 或 `en_dict.txt` 字符集文件，也可放在模型目录下。
如果没有，程序会使用内置的默认字符集（数字+字母+常用符号）。
