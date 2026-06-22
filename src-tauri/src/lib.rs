use anyhow::{Context, Result};
use base64::Engine;
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{Emitter, Manager};
use walkdir::WalkDir;

struct AppState;

#[derive(Clone, Serialize)]
struct OcrProgress { current: usize, total: usize, matched: usize, unmatched: usize, current_file: String, matched_number: Option<String>, ocr_text: String }

#[derive(Clone, Serialize)]
struct OcrDone { total: usize, matched: usize, unmatched: usize, elapsed: u64 }

// ─── Python OCR 调用 ───────────────────────────────────────

fn ocr_image(img_path: &Path) -> Result<String> {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("ocr_helper.py");
    if !script.exists() { return Err(anyhow::anyhow!("ocr_helper.py 不存在")); }
    let path = img_path.to_string_lossy().to_string();
    let out = std::process::Command::new("python").arg(&script).arg(&path).output().context("调用 Python 失败")?;
    if !out.status.success() { return Err(anyhow::anyhow!("Python 错误: {}", String::from_utf8_lossy(&out.stderr))); }
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).context("解析 JSON 失败")?;
    if let Some(e) = parsed[&path]["error"].as_str() { return Err(anyhow::anyhow!("OCR 错误: {}", e)); }
    Ok(parsed[&path]["text"].as_str().unwrap_or("").to_string())
}

// ─── 编号匹配 ───────────────────────────────────────────────

fn find_number(text: &str, numbers: &[String], trailing: usize) -> Option<String> {
    let c: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if let Ok(re) = Regex::new(r"\d+") {
        let all: String = re.find_iter(&c).map(|m| m.as_str()).collect();
        for num in numbers {
            let d: String = num.chars().filter(|c| c.is_ascii_digit()).collect();
            if d.len() >= trailing {
                let t = &d[d.len() - trailing..];
                if all.contains(t) { return Some(num.clone()); }
            }
        }
    }
    None
}

// ─── 收集图片 ───────────────────────────────────────────────

fn collect_images(dir: &Path) -> Vec<PathBuf> {
    let exts = ["jpg", "jpeg", "png", "bmp", "tiff", "tif", "webp"];
    WalkDir::new(dir).max_depth(1).into_iter().filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()).filter(|e| {
        e.path().extension().and_then(|ext| ext.to_str()).map(|ext| exts.contains(&ext.to_lowercase().as_str())).unwrap_or(false)
    }).map(|e| e.path().to_path_buf()).collect()
}

// ─── Tauri Commands ─────────────────────────────────────────

#[tauri::command]
async fn select_folder() -> Result<Option<String>, String> {
    Ok(rfd::AsyncFileDialog::new().set_title("选择文件夹").pick_folder().await.map(|p| p.path().to_string_lossy().into_owned()))
}

#[tauri::command]
fn get_qrcode(ah: tauri::AppHandle, name: String) -> Result<Option<String>, String> {
    let dir = if cfg!(debug_assertions) { PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("assets/qrcode") }
        else { ah.path().resource_dir().map(|p| p.join("assets/qrcode")).unwrap_or_else(|_| PathBuf::from("assets/qrcode")) };
    let fp = dir.join(&name); if !fp.exists() { return Ok(None); }
    let d = fs::read(&fp).map_err(|e| e.to_string())?;
    let m = match fp.extension().and_then(|e| e.to_str()) { Some("png") => "image/png", Some("jpg")|Some("jpeg") => "image/jpeg", _ => "image/png" };
    Ok(Some(format!("data:{};base64,{}", m, base64::engine::general_purpose::STANDARD.encode(&d))))
}

#[tauri::command]
fn list_folders(path: String) -> Result<Vec<String>, String> {
    let dir = Path::new(&path); if !dir.is_dir() { return Err("".to_string()); }
    let mut f: Vec<String> = fs::read_dir(dir).map_err(|e| e.to_string())?.filter_map(|e| e.ok()).filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false)).filter_map(|e| e.file_name().to_str().map(|s| s.to_string())).collect();
    f.sort(); Ok(f)
}

#[tauri::command]
async fn start_ocr(image_dir: String, output_dir: Option<String>, numbers: Vec<String>, trailing_digits: usize, ah: tauri::AppHandle) -> Result<(), String> {
    let ip = Path::new(&image_dir); if !ip.is_dir() { return Err("图片目录无效".to_string()); }
    let op = match output_dir { Some(p) if !p.is_empty() => PathBuf::from(p), _ => ip.join("output") };
    fs::create_dir_all(&op).map_err(|e| e.to_string())?;
    for n in &numbers { fs::create_dir_all(&op.join(n)).map_err(|e| e.to_string())?; }
    let uq = op.join("模糊图片"); fs::create_dir_all(&uq).map_err(|e| e.to_string())?;
    let imgs = collect_images(ip); let total = imgs.len();
    if total == 0 { return Err("未找到图片".to_string()); }
    let t0 = Instant::now(); let mc = Arc::new(AtomicUsize::new(0)); let uc = Arc::new(AtomicUsize::new(0));

    for (i, img) in imgs.iter().enumerate() {
        let fn_ = img.file_name().unwrap_or_default().to_string_lossy().to_string();
        ah.emit("ocr-progress", OcrProgress { current: i+1, total, matched: mc.load(Ordering::SeqCst), unmatched: uc.load(Ordering::SeqCst), current_file: fn_.clone(), matched_number: None, ocr_text: String::new() }).ok();

        match ocr_image(img) {
            Ok(t) => {
                log::info!("[OCR] {} → {}", fn_, t);
                if let Some(n) = find_number(&t, &numbers, trailing_digits) {
                    fs::copy(img, &op.join(&n).join(&fn_)).map_err(|e| e.to_string())?;
                    mc.fetch_add(1, Ordering::SeqCst);
                    ah.emit("ocr-progress", OcrProgress { current: i+1, total, matched: mc.load(Ordering::SeqCst), unmatched: uc.load(Ordering::SeqCst), current_file: fn_.clone(), matched_number: Some(n), ocr_text: t }).ok();
                } else { fs::copy(img, &uq.join(&fn_)).ok(); uc.fetch_add(1, Ordering::SeqCst); }
            }
            Err(e) => { log::error!("[OCR] {} 失败: {}", fn_, e); fs::copy(img, &uq.join(&fn_)).ok(); uc.fetch_add(1, Ordering::SeqCst); }
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }

    ah.emit("ocr-done", OcrDone { total, matched: mc.load(Ordering::SeqCst), unmatched: uc.load(Ordering::SeqCst), elapsed: t0.elapsed().as_secs() }).ok();
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default().setup(|app| {
        if cfg!(debug_assertions) { app.handle().plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())?; }
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("ocr_helper.py");
        if script.exists() { log::info!("ocr_helper.py 已就绪"); }
        match std::process::Command::new("python").arg("--version").output() {
            Ok(o) if o.status.success() => log::info!("Python: {}", String::from_utf8_lossy(&o.stdout).trim()),
            _ => log::warn!("Python 不可用"),
        }
        app.manage(AppState);
        Ok(())
    }).invoke_handler(tauri::generate_handler![start_ocr, list_folders, select_folder, get_qrcode]).run(tauri::generate_context!()).expect("error running app");
}