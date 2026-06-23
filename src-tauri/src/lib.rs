use base64::Engine;
use image::GenericImageView;
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{Emitter, Manager};
use walkdir::WalkDir;

struct AppState {
    engine: OcrEngine,
}

#[derive(Clone, Serialize)]
struct OcrProgress { current: usize, total: usize, matched: usize, unmatched: usize, current_file: String, matched_number: Option<String>, ocr_text: String }

#[derive(Clone, Serialize)]
struct OcrDone { total: usize, matched: usize, unmatched: usize, elapsed: u64 }

fn get_model_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        let p = exe.parent().unwrap().join("models");
        if p.join("text-detection.rten").exists() { return p; }
    }
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models");
    if p.join("text-detection.rten").exists() { return p; }
    PathBuf::from("models") // 打包后资源目录
}

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

fn collect_images(dir: &Path) -> Vec<PathBuf> {
    let exts = ["jpg", "jpeg", "png", "bmp", "tiff", "tif", "webp"];
    WalkDir::new(dir).max_depth(1).into_iter().filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()).filter(|e| {
        e.path().extension().and_then(|ext| ext.to_str()).map(|ext| exts.contains(&ext.to_lowercase().as_str())).unwrap_or(false)
    }).map(|e| e.path().to_path_buf()).collect()
}

#[tauri::command]
async fn select_folder() -> Result<Option<String>, String> {
    Ok(rfd::AsyncFileDialog::new().set_title("选择文件夹").pick_folder().await.map(|p| p.path().to_string_lossy().into_owned()))
}

#[tauri::command]
fn get_qrcode(ah: tauri::AppHandle, name: String) -> Result<Option<String>, String> {
    let paths = if cfg!(debug_assertions) {
        vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/qrcode")]
    } else {
        let rd = ah.path().resource_dir().unwrap_or_default();
        vec![
            rd.join("assets/qrcode"),
            rd.join("qrcode"),
            rd.clone(),
            PathBuf::from("assets/qrcode"),
        ]
    };
    for dir in &paths {
        let fp = dir.join(&name);
        log::info!("get_qrcode 尝试: {:?}", fp);
        if fp.exists() {
            let d = fs::read(&fp).map_err(|e| e.to_string())?;
            let m = match fp.extension().and_then(|e| e.to_str()) { Some("png") => "image/png", Some("jpg")|Some("jpeg") => "image/jpeg", _ => "image/png" };
            return Ok(Some(format!("data:{};base64,{}", m, base64::engine::general_purpose::STANDARD.encode(&d))));
        }
    }
    Ok(None)
}

#[tauri::command]
fn list_folders(path: String) -> Result<Vec<String>, String> {
    let dir = Path::new(&path);
    if !dir.is_dir() { return Err("".to_string()); }
    let mut f: Vec<String> = fs::read_dir(dir).map_err(|e| e.to_string())?.filter_map(|e| e.ok()).filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false)).filter_map(|e| e.file_name().to_str().map(|s| s.to_string())).collect();
    f.sort(); Ok(f)
}

#[tauri::command]
async fn start_ocr(image_dir: String, output_dir: Option<String>, numbers: Vec<String>, trailing_digits: usize, ah: tauri::AppHandle, st: tauri::State<'_, AppState>) -> Result<(), String> {
    let ip = Path::new(&image_dir);
    if !ip.is_dir() { return Err("图片目录无效".to_string()); }
    let op = match output_dir { Some(p) if !p.is_empty() => PathBuf::from(p), _ => ip.join("output") };
    fs::create_dir_all(&op).map_err(|e| e.to_string())?;
    for n in &numbers { fs::create_dir_all(&op.join(n)).map_err(|e| e.to_string())?; }
    let uq = op.join("模糊或没对应编码图片"); fs::create_dir_all(&uq).map_err(|e| e.to_string())?;
    let imgs = collect_images(ip); let total = imgs.len();
    if total == 0 { return Err("未找到图片".to_string()); }
    let t0 = Instant::now(); let mc = Arc::new(AtomicUsize::new(0)); let uc = Arc::new(AtomicUsize::new(0));

    for (i, img) in imgs.iter().enumerate() {
        let fn_ = img.file_name().unwrap_or_default().to_string_lossy().to_string();
        ah.emit("ocr-progress", OcrProgress { current: i+1, total, matched: mc.load(Ordering::SeqCst), unmatched: uc.load(Ordering::SeqCst), current_file: fn_.clone(), matched_number: None, ocr_text: String::new() }).ok();

        let result = (|| -> anyhow::Result<String> {
            let img = image::open(img).map_err(|e| anyhow::anyhow!("打开图片失败: {}", e))?;
            let rgb = img.to_rgb8();
            let rgb = if rgb.width().max(rgb.height()) > 1500 {
                let r = 1500.0 / rgb.width().max(rgb.height()) as f64;
                let (nw, nh) = ((rgb.width() as f64 * r) as u32, (rgb.height() as f64 * r) as u32);
                image::imageops::resize(&rgb, nw, nh, image::imageops::FilterType::Triangle)
            } else { rgb };
            let (w, h) = rgb.dimensions();
            let pixels = rgb.into_raw();
            let src = ImageSource::from_bytes(&pixels, (w, h)).map_err(|e| anyhow::anyhow!("{:?}", e))?;
            let input = st.engine.prepare_input(src)?;
            let text = st.engine.get_text(&input)?;
            Ok(text.chars().filter(|c| c.is_ascii_digit() || c.is_whitespace()).collect())
        })();

        match result {
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
        let md = get_model_dir();
        let (dp, rp) = if md.join("text-detection.rten").exists() {
            (md.join("text-detection.rten"), md.join("text-recognition.rten"))
        } else if let Ok(rd) = app.path().resource_dir() {
            (rd.join("models/text-detection.rten"), rd.join("models/text-recognition.rten"))
        } else {
            return Err("模型文件无法定位".into());
        };
        if !dp.exists() || !rp.exists() { return Err("模型文件缺失".into()); }
        log::info!("加载检测模型...");
        let det = rten::Model::load_file(&dp).map_err(|e| format!("检测模型加载失败: {}", e))?;
        log::info!("加载识别模型...");
        let rec = rten::Model::load_file(&rp).map_err(|e| format!("识别模型加载失败: {}", e))?;
        let engine = OcrEngine::new(OcrEngineParams {
            detection_model: Some(det),
            recognition_model: Some(rec),
            allowed_chars: Some("0123456789".to_string()),
            ..Default::default()
        }).map_err(|e| format!("引擎初始化失败: {}", e))?;
        app.manage(AppState { engine });
        Ok(())
    }).invoke_handler(tauri::generate_handler![start_ocr, list_folders, select_folder, get_qrcode]).run(tauri::generate_context!()).expect("error running app");
}