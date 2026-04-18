// Tauri命令层 - 薄层，仅负责参数转换和调用Service层
// 所有业务逻辑下沉到Service层

use tauri::State;
use std::sync::Mutex;
use std::path::PathBuf;

use crate::models::*;
use crate::services::{StorageService, DecoderService, EncoderService};
use crate::dcc::{ExportConfig, ExportFormat, FrameRange, generate_maya_import_script, generate_blender_import_script};
use image::GenericImageView;

// =============================================================================
// 应用状态
// =============================================================================

pub struct AppState {
    pub storage: Mutex<StorageService>,
    pub decoder: DecoderService,
    pub encoder: EncoderService,
}

// =============================================================================
// 素材管理命令
// =============================================================================

/// 创建素材（导入并解码）
#[tauri::command]
pub fn create_asset(
    state: State<AppState>,
    source_path: String,
) -> Result<Asset, String> {
    let mut storage = state.storage.lock().map_err(|e| e.to_string())?;
    
    let path = PathBuf::from(&source_path);
    
    // 探测素材信息
    let (format, width, height, fps) = probe_asset_info(&path)
        .map_err(|e| format!("Failed to probe asset: {}", e))?;
    
    let asset_id = uuid::Uuid::new_v4().to_string();
    let frame_dir = storage.get_frame_dir(&asset_id);
    
    let asset = Asset {
        id: asset_id.clone(),
        source: AssetSource::Local,
        source_url: None,
        format,
        frame_count: 0,
        fps: fps as f32,
        resolution: Resolution::new(width, height),
        original_path: path.clone(),
        frame_directory: frame_dir.clone(),
        thumbnail_path: frame_dir.join("thumbnail.jpg"),
        tags: vec![],
        imported_at: chrono::Utc::now(),
        modified_at: chrono::Utc::now(),
        view_count: 0,
        last_viewed_at: None,
    };
    
    // 保存到数据库
    storage.create_asset(&asset).map_err(|e| e.to_string())?;
    
    // 解码素材为帧序列
    let frames = state.decoder.decode_asset(&asset)
        .map_err(|e| format!("Failed to decode: {}", e))?;
    
    // 更新帧数
    let mut updated_asset = asset;
    updated_asset.frame_count = frames.len() as u32;
    storage.update_asset(&updated_asset).map_err(|e| e.to_string())?;
    
    Ok(updated_asset)
}

/// 获取单个素材
#[tauri::command]
pub fn get_asset(
    state: State<AppState>,
    id: String,
) -> Result<Option<Asset>, String> {
    let mut storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.get_asset(&id).map_err(|e| e.to_string())
}

/// 获取所有素材列表
#[tauri::command]
pub fn get_all_assets(
    state: State<AppState>,
) -> Result<Vec<Asset>, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.get_all_assets().map_err(|e| e.to_string())
}

/// 删除素材
#[tauri::command]
pub fn delete_asset(
    state: State<AppState>,
    id: String,
) -> Result<bool, String> {
    let mut storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.delete_asset(&id).map_err(|e| e.to_string())
}

// =============================================================================
// 帧管理命令
// =============================================================================

/// 获取帧文件路径
#[tauri::command]
pub fn get_frame_path(
    state: State<AppState>,
    asset_id: String,
    frame_index: u32,
) -> Result<String, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let path = storage.get_frame_path(&asset_id, frame_index);
    Ok(path.to_string_lossy().to_string())
}

/// 获取帧数据 (base64编码图像)
#[tauri::command]
pub fn get_frame_data(
    state: State<AppState>,
    asset_id: String,
    frame_index: u32,
) -> Result<String, String> {
    use base64::Engine;
    
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    
    let frame_path = storage.get_frame_path(&asset_id, frame_index);
    
    if !frame_path.exists() {
        return Err(format!("Frame {} not found", frame_index));
    }
    
    let data = std::fs::read(&frame_path).map_err(|e| e.to_string())?;
    let base64 = base64::engine::general_purpose::STANDARD.encode(&data);
    
    Ok(format!("data:image/png;base64,{}", base64))
}

// =============================================================================
// 标注管理命令
// =============================================================================

/// 创建标注
#[tauri::command]
pub fn create_annotation(
    state: State<AppState>,
    asset_id: String,
    frame_index: Option<u32>,
    annotation_type: AnnotationType,
    coordinates: AnnotationCoords,
    label: Option<String>,
    color: String,
) -> Result<Annotation, String> {
    let mut storage = state.storage.lock().map_err(|e| e.to_string())?;
    
    let annotation = Annotation {
        id: uuid::Uuid::new_v4().to_string(),
        asset_id,
        frame_index,
        annotation_type,
        coordinates,
        label,
        color,
        created_at: chrono::Utc::now(),
        created_by: "user".to_string(),
        metadata: None,
    };
    
    storage.create_annotation(&annotation).map_err(|e| e.to_string())?;
    
    Ok(annotation)
}

/// 获取素材的所有标注
#[tauri::command]
pub fn get_annotations(
    state: State<AppState>,
    asset_id: String,
) -> Result<Vec<Annotation>, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.get_annotations_for_asset(&asset_id).map_err(|e| e.to_string())
}

/// 获取特定帧的标注
#[tauri::command]
pub fn get_annotations_for_frame(
    state: State<AppState>,
    asset_id: String,
    frame_index: u32,
) -> Result<Vec<Annotation>, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.get_annotations_for_frame(&asset_id, frame_index).map_err(|e| e.to_string())
}

/// 删除标注
#[tauri::command]
pub fn delete_annotation(
    state: State<AppState>,
    annotation_id: String,
) -> Result<bool, String> {
    let mut storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.delete_annotation(&annotation_id).map_err(|e| e.to_string())
}

// =============================================================================
// 导出命令
// =============================================================================

/// 导出素材
#[tauri::command]
pub fn export_asset(
    state: State<AppState>,
    asset_id: String,
    config: ExportConfig,
) -> Result<String, String> {
    let mut storage = state.storage.lock().map_err(|e| e.to_string())?;
    
    let asset = storage.get_asset(&asset_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Asset not found".to_string())?;
    
    let output_path = PathBuf::from(&config.output_path);
    
    // 确保输出目录存在
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    
    // 执行导出
    match config.format.clone() {
        ExportFormat::PngSequence => {
            export_png_sequence(&asset, &output_path, &config.frame_range)?;
        }
        _ => {
            // 使用EncoderService进行视频编码
            state.encoder.encode_with_annotations(
                &asset,
                &output_path,
                config.format.clone().into(),
                config.frame_range.clone().into(),
                config.include_annotations,
            ).map_err(|e| format!("Export failed: {}", e))?;
        }
    }
    
    // 创建导出记录
    let record = ExportRecord {
        id: uuid::Uuid::new_v4().to_string(),
        asset_id: asset_id.clone(),
        format: config.format.into(),
        frame_range: config.frame_range.into(),
        include_annotations: config.include_annotations,
        output_path: output_path.clone(),
        exported_at: chrono::Utc::now(),
        file_size_bytes: std::fs::metadata(&output_path).map(|m| m.len()).unwrap_or(0),
    };
    
    // 保存记录（需要重新获取storage锁）
    drop(storage);
    let mut storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.create_export_record(&record).map_err(|e| e.to_string())?;
    
    Ok(output_path.to_string_lossy().to_string())
}

/// 生成DCC导入辅助脚本
#[tauri::command]
pub fn generate_import_script(
    dcc: String,
    sequence_path: String,
    fps: f32,
    frame_count: u32,
) -> Result<String, String> {
    let path = PathBuf::from(sequence_path);
    
    let script = match dcc.as_str() {
        "maya" => generate_maya_import_script(&path, fps),
        "blender" => generate_blender_import_script(&path, frame_count),
        _ => return Err(format!("Unsupported DCC: {}", dcc)),
    };
    
    Ok(script)
}

// =============================================================================
// 内部辅助函数
// =============================================================================

fn probe_asset_info(path: &PathBuf) -> anyhow::Result<(AssetFormat, u32, u32, f64)> {
    use std::process::Command;
    
    // 根据扩展名判断格式
    let format = match path.extension().and_then(|e| e.to_str()) {
        Some("gif") => AssetFormat::Gif,
        Some("webm") => AssetFormat::Webm,
        Some("mp4") => AssetFormat::Mp4,
        Some("mov") => AssetFormat::Mov,
        Some("png") => AssetFormat::Png,
        Some("jpg") | Some("jpeg") => AssetFormat::Jpg,
        _ => return Err(anyhow::anyhow!("Unknown file format")),
    };

    // 对于静态图片
    if format == AssetFormat::Png || format == AssetFormat::Jpg {
        let img = image::open(path)?;
        let (w, h) = img.dimensions();
        return Ok((format, w, h, 30.0));
    }

    // 对于GIF和视频，使用FFmpeg探测
    let output = Command::new("ffprobe")
        .args(&[
            "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=width,height,r_frame_rate",
            "-of", "csv=s=x:p=0",
            path.to_str().unwrap()
        ])
        .output()?;

    let info = String::from_utf8(output.stdout)?;
    let parts: Vec<&str> = info.trim().split('x').collect();
    
    if parts.len() >= 2 {
        let width = parts[0].parse::<u32>()?;
        let height_parts: Vec<&str> = parts[1].split('/').collect();
        let height = height_parts[0].parse::<u32>()?;
        
        // 解析帧率
        let fps = if parts.len() >= 3 {
            let num = parts[1].parse::<f64>().unwrap_or(30000.0);
            let den = parts[2].parse::<f64>().unwrap_or(1001.0);
            num / den
        } else {
            30.0
        };

        Ok((format, width, height, fps))
    } else {
        Err(anyhow::anyhow!("Failed to probe asset info"))
    }
}

fn export_png_sequence(
    asset: &Asset,
    output_dir: &PathBuf,
    frame_range: &FrameRange,
) -> Result<(), String> {
    std::fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
    
    let start = frame_range.start(asset.frame_count);
    let end = frame_range.end(asset.frame_count);
    
    for i in start..=end {
        let src = asset.frame_directory.join(format!("frame_{:06}.png", i));
        let dst = output_dir.join(format!("frame_{:06}.png", i));
        if src.exists() {
            std::fs::copy(&src, &dst).map_err(|e| e.to_string())?;
        }
    }
    
    Ok(())
}

// 类型转换: dcc::ExportFormat -> models::ExportFormat
impl From<ExportFormat> for crate::models::ExportFormat {
    fn from(f: ExportFormat) -> Self {
        match f {
            ExportFormat::PngSequence => crate::models::ExportFormat::ImageSequence,
            ExportFormat::H264 => crate::models::ExportFormat::H264 { quality: crate::models::H264Quality::High },
            ExportFormat::ProRes => crate::models::ExportFormat::ProRes,
            ExportFormat::Gif => crate::models::ExportFormat::Gif,
            ExportFormat::WebM => crate::models::ExportFormat::Webm,
        }
    }
}

// 类型转换: dcc::FrameRange -> models::FrameRange
impl From<FrameRange> for crate::models::FrameRange {
    fn from(r: FrameRange) -> Self {
        match r {
            FrameRange::All => crate::models::FrameRange::All,
            FrameRange::Custom { start, end } => crate::models::FrameRange::Custom { start, end },
        }
    }
}

// =============================================================================
// Sakugabooru 搜索和下载命令
// =============================================================================

use crate::services::{SakugabooruClient, SakugaPost, SearchOptions};

/// 搜索Sakugabooru
#[tauri::command]
pub async fn search_sakugabooru(
    query: String,
    page: u32,
    limit: u32,
) -> Result<Vec<SakugaPost>, String> {
    let client = SakugabooruClient::new();
    let options = SearchOptions {
        query,
        page,
        limit,
    };
    
    client.search(&options).await.map_err(|e| e.to_string())
}

/// 下载Sakugabooru帖子并导入为素材
#[tauri::command]
pub async fn download_sakuga_post(
    state: State<'_, AppState>,
    post: SakugaPost,
) -> Result<Asset, String> {
    let client = SakugabooruClient::new();
    
    // 创建临时下载目录
    let temp_dir = state.storage.lock().map_err(|e| e.to_string())?.get_temp_dir();
    let file_ext = PathBuf::from(&post.file_url)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("webm")
        .to_string();
    
    let download_path = temp_dir.join(format!("sakuga_{}.{}.{}", post.id, uuid::Uuid::new_v4(), file_ext));
    
    // 下载文件
    client.download_file(&post.file_url, &download_path).await
        .map_err(|e| format!("Download failed: {}", e))?;
    
    // 导入为素材（复用create_asset逻辑）
    let path = download_path.clone();
    let mut storage = state.storage.lock().map_err(|e| e.to_string())?;
    
    // 探测素材信息
    let (format, width, height, fps) = probe_asset_info(&path)
        .map_err(|e| format!("Failed to probe asset: {}", e))?;
    
    let asset_id = uuid::Uuid::new_v4().to_string();
    let frame_dir = storage.get_frame_dir(&asset_id);
    
    // 解析Sakugabooru标签
    let tags = crate::services::sakugabooru_client::parse_sakuga_tags(&post.tags);
    
    let asset = Asset {
        id: asset_id.clone(),
        source: AssetSource::Sakugabooru,
        source_url: Some(format!("https://www.sakugabooru.com/post/show/{}", post.id)),
        format,
        frame_count: 0,
        fps: fps as f32,
        resolution: Resolution::new(width, height),
        original_path: path.clone(),
        frame_directory: frame_dir.clone(),
        thumbnail_path: frame_dir.join("thumbnail.jpg"),
        tags: tags.into_iter().map(|(name, _)| Tag { name, category: TagCategory::General, confidence: None }).collect(),
        imported_at: chrono::Utc::now(),
        modified_at: chrono::Utc::now(),
        view_count: 0,
        last_viewed_at: None,
    };
    
    // 保存到数据库
    storage.create_asset(&asset).map_err(|e| e.to_string())?;
    
    // 解码素材为帧序列
    drop(storage); // 释放锁
    let frames = state.decoder.decode_asset(&asset)
        .map_err(|e| format!("Failed to decode: {}", e))?;
    
    // 更新帧数
    let mut storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut updated_asset = asset;
    updated_asset.frame_count = frames.len() as u32;
    storage.update_asset(&updated_asset).map_err(|e| e.to_string())?;
    
    Ok(updated_asset)
}

/// 获取Sakugabooru帖子详情
#[tauri::command]
pub async fn get_sakuga_post(
    post_id: u32,
) -> Result<SakugaPost, String> {
    let client = SakugabooruClient::new();
    client.get_post(post_id).await.map_err(|e| e.to_string())
}