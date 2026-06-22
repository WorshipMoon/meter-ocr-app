"""RapidOCR 识别辅助脚本 - 由 Rust 后端调用"""
import sys, json, traceback
from rapidocr_onnxruntime import RapidOCR
engine = RapidOCR()

def ocr(path: str) -> dict:
    try:
        result, elapse = engine(path)
        if result is None: return {"text": ""}
        texts = [line[1] for line in result]
        return {"text": " ".join(texts)}
    except: return {"error": traceback.format_exc()}

if __name__ == "__main__":
    if len(sys.argv) < 2: print(json.dumps({"error": "need args"})); sys.exit(1)
    res = {}
    for p in sys.argv[1:]: res[p] = ocr(p)
    print(json.dumps(res, ensure_ascii=False))
