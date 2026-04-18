//! 核心数据模型 - Deep Module: 定义所有领域实体
//! 
//! 设计原则:
//! - 使用强类型避免非法状态
//! - 相对路径存储，便于迁移
//! - 坐标归一化(0-1)，适配不同分辨率

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 素材唯一标识
pub type AssetId = String;
pub type AnnotationId = String;

/// 素材实体 - 核心业务对象
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: AssetId,
    pub source: AssetSource,
    pub source_url: Option<String>,
    
    // 媒体规格 (只读，创建后不变)
    pub format: AssetFormat,
    pub frame_count: u32,
    pub fps: f32,
    pub resolution: Resolution,
    
    // 存储路径 (相对于应用数据目录)
    pub original_path: PathBuf,
    pub frame_directory: PathBuf,
    pub thumbnail_path: PathBuf,
    
    // 元数据
    pub tags: Vec<Tag>,
    pub imported_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    
    // 统计
    pub view_count: u32,
    pub last_viewed_at: Option<DateTime<Utc>>,
}

/// 素材来源
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetSource {
    Sakugabooru,
    Local,
    Url,
}

/// 素材格式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetFormat {
    Gif,
    Webm,
    Mp4,
    Mov,
    ImageSequence,
    Png,
    Jpg,
}

/// 分辨率
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    /// 创建新分辨率
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
    
    /// 计算归一化坐标对应的像素值
    pub fn denormalize(&self, x: f32, y: f32) -> (i32, i32) {
        (
            (x * self.width as f32) as i32,
            (y * self.height as f32) as i32,
        )
    }
    
    /// 将像素坐标归一化
    pub fn normalize(&self, x: i32, y: i32) -> (f32, f32) {
        (
            x as f32 / self.width as f32,
            y as f32 / self.height as f32,
        )
    }
}

/// 标签
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub category: TagCategory,
    pub confidence: Option<f32>,
}

/// 标签分类 (Booru兼容)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TagCategory {
    Character,
    Copyright,
    Artist,
    General,
    Meta,
}

/// 帧信息 (轻量引用，实际数据在文件系统)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub asset_id: AssetId,
    pub index: u32,
    pub timestamp_ms: u64,
    pub filename: String,
    pub has_annotation: bool,
}

impl Frame {
    /// 生成标准帧文件名
    pub fn filename_for_index(index: u32) -> String {
        format!("frame_{:06}.png", index)
    }
}

/// 标注类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationType {
    Rect,
    Circle,
    Arrow,
    Text,
    Stroke,  // 手绘笔迹
}

/// 标注实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub id: AnnotationId,
    pub asset_id: AssetId,
    /// 关联帧，None表示跨所有帧(global)
    pub frame_index: Option<u32>,
    pub annotation_type: AnnotationType,
    pub coordinates: AnnotationCoords,
    pub label: Option<String>,
    pub color: String,  // hex: #RRGGBB
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub metadata: Option<serde_json::Value>,
}

/// 标注坐标 (归一化0-1，适配不同分辨率)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnnotationCoords {
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    Circle {
        cx: f32,
        cy: f32,
        r: f32,
    },
    Arrow {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    },
    Text {
        x: f32,
        y: f32,
    },
    Stroke {
        points: Vec<(f32, f32)>,
    },
}

/// 导出记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRecord {
    pub id: String,
    pub asset_id: AssetId,
    pub format: ExportFormat,
    pub frame_range: FrameRange,
    pub include_annotations: bool,
    pub output_path: PathBuf,
    pub exported_at: DateTime<Utc>,
    pub file_size_bytes: u64,
}

/// 导出格式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    H264 { quality: H264Quality },
    ProRes,
    Gif,
    Webm,
    ImageSequence,
    // DCC专用
    MayaPlayblast,
    BlenderViewport,
}

/// H264质量预设
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum H264Quality {
    Lossless,  // CRF 0
    High,      // CRF 18
    Medium,    // CRF 23
    Low,       // CRF 28
}

impl Default for H264Quality {
    fn default() -> Self {
        H264Quality::High
    }
}

/// 帧范围
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FrameRange {
    All,
    Custom { start: u32, end: u32 },
}

impl FrameRange {
    /// 检查帧是否在范围内 (Define Errors Out: 自动处理边界)
    pub fn contains(&self, frame: u32, total_frames: u32) -> bool {
        match self {
            FrameRange::All => frame < total_frames,
            FrameRange::Custom { start, end } => {
                let end = (*end).min(total_frames.saturating_sub(1));
                frame >= *start && frame <= end
            }
        }
    }
    
    /// 获取实际起始帧 (Define Errors Out: 越界时返回有效值)
    pub fn start(&self, total_frames: u32) -> u32 {
        match self {
            FrameRange::All => 0,
            FrameRange::Custom { start, .. } => (*start).min(total_frames.saturating_sub(1)),
        }
    }
    
    /// 获取实际结束帧
    pub fn end(&self, total_frames: u32) -> u32 {
        match self {
            FrameRange::All => total_frames.saturating_sub(1),
            FrameRange::Custom { end, .. } => (*end).min(total_frames.saturating_sub(1)),
        }
    }
}

/// 素材查询过滤器
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetFilter {
    pub tags: Vec<String>,
    pub format: Option<AssetFormat>,
    pub source: Option<AssetSource>,
    pub has_annotations: Option<bool>,
    pub imported_after: Option<DateTime<Utc>>,
    pub imported_before: Option<DateTime<Utc>>,
    pub search_text: Option<String>,
}

/// 素材排序
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetSort {
    ImportedAtDesc,
    ImportedAtAsc,
    NameAsc,
    NameDesc,
    LastViewedDesc,
    ViewCountDesc,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resolution_normalize() {
        let res = Resolution::new(1920, 1080);
        assert_eq!(res.normalize(960, 540), (0.5, 0.5));
        assert_eq!(res.denormalize(0.5, 0.5), (960, 540));
    }
    
    #[test]
    fn test_frame_range_bounds() {
        // Define Errors Out: 越界自动裁剪
        let range = FrameRange::Custom { start: 0, end: 1000 };
        assert_eq!(range.end(100), 99);  // 自动裁剪到实际帧数
        
        let range = FrameRange::Custom { start: 50, end: 60 };
        assert!(range.contains(55, 100));
        assert!(!range.contains(45, 100));  // 低于起始
        assert!(!range.contains(65, 100));  // 高于结束
    }
    
    #[test]
    fn test_frame_filename() {
        assert_eq!(Frame::filename_for_index(0), "frame_000000.png");
        assert_eq!(Frame::filename_for_index(42), "frame_000042.png");
        assert_eq!(Frame::filename_for_index(999999), "frame_999999.png");
    }
}
