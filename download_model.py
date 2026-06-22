#!/usr/bin/env python3
"""
下载 PP-OCRv4 英文识别模型到项目目录
"""
import os
import sys
import urllib.request
import zipfile
import shutil

MODEL_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), ".rapidocr_onnxruntime", "models")
MODEL_FILE = os.path.join(MODEL_DIR, "en_PP-OCRv4_rec_infer.onnx")

# RapidOCR 官方模型下载地址
DOWNLOAD_URLS = [
    "https://github.com/RapidAI/RapidOCR/releases/download/v0.0.0/en_PP-OCRv4_rec_infer.onnx",
    "https://huggingface.co/RapidAI/PP-OCRv4/resolve/main/en_PP-OCRv4_rec_infer.onnx",
]

def download_file(url, dest):
    print(f"正在下载: {url}")
    try:
        urllib.request.urlretrieve(url, dest)
        size = os.path.getsize(dest)
        print(f"下载完成: {size / 1024 / 1024:.1f} MB")
        return True
    except Exception as e:
        print(f"下载失败: {e}")
        if os.path.exists(dest):
            os.remove(dest)
        return False

def main():
    os.makedirs(MODEL_DIR, exist_ok=True)

    if os.path.exists(MODEL_FILE):
        print(f"模型文件已存在: {MODEL_FILE}")
        return

    print(f"模型目录: {MODEL_DIR}")
    print(f"目标文件: {MODEL_FILE}")
    print()

    for url in DOWNLOAD_URLS:
        if download_file(url, MODEL_FILE):
            print("模型下载成功！")
            return

    print()
    print("所有下载方式均失败，请手动下载：")
    print("1. 访问 https://github.com/RapidAI/RapidOCR/releases")
    print("2. 下载 en_PP-OCRv4_rec_infer.onnx")
    print(f"3. 将文件放到: {MODEL_DIR}")
    sys.exit(1)

if __name__ == "__main__":
    main()
