//! 存储服务 - Deep Module
//! 
//! 提供简洁接口，隐藏SQLite和文件系统的复杂性。
//! 所有路径都相对于应用数据目录，便于迁移。

use std::path::{Path, PathBuf};
use std::fs;
use chrono::{Utc, DateTime};
use rusqlite::{Connection, OptionalExtension, params};
use anyhow::{Result, Context};
use uuid::Uuid;

use crate::models::*;

/// 存储服务 - 管理所有持久化数据
/// 
/// 接口原则: 简洁方法名，复杂实现隐藏
pub struct StorageService {
    db: Connection,
    data_dir: PathBuf,
    assets_dir: PathBuf,
    frames_dir: PathBuf,
}

impl StorageService {
    /// 初始化存储服务
    pub fn new(data_dir: impl AsRef<Path>) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        let assets_dir = data_dir.join("assets");
        let frames_dir = data_dir.join("frames");
        
        // 创建目录结构
        fs::create_dir_all(&assets_dir)?;
        fs::create_dir_all(&frames_dir)?;
        
        // 初始化数据库
        let db_path = data_dir.join("aam.db");
        let db = Connection::open(&db_path)?;
        
        let mut service = Self {
            db,
            data_dir,
            assets_dir,
            frames_dir,
        };
        
        service.init_schema()?;
        
        Ok(service)
    }
    
    /// 初始化数据库表结构
    fn init_schema(&mut self) -> Result<()> {
        // Assets表
        self.db.execute(
            "CREATE TABLE IF NOT EXISTS assets (
                id TEXT PRIMARY KEY,
                source TEXT NOT NULL,
                source_url TEXT,
                format TEXT NOT NULL,
                frame_count INTEGER NOT NULL,
                fps REAL NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                original_path TEXT NOT NULL,
                frame_directory TEXT NOT NULL,
                thumbnail_path TEXT NOT NULL,
                tags TEXT NOT NULL,
                imported_at TEXT NOT NULL,
                modified_at TEXT NOT NULL,
                view_count INTEGER DEFAULT 0,
                last_viewed_at TEXT
            )",
            [],
        )?;
        
        // Annotations表
        self.db.execute(
            "CREATE TABLE IF NOT EXISTS annotations (
                id TEXT PRIMARY KEY,
                asset_id TEXT NOT NULL,
                frame_index INTEGER,
                annotation_type TEXT NOT NULL,
                coordinates TEXT NOT NULL,
                label TEXT,
                color TEXT NOT NULL,
                created_at TEXT NOT NULL,
                created_by TEXT NOT NULL,
                metadata TEXT,
                FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE
            )",
            [],
        )?;
        
        // Export records表
        self.db.execute(
            "CREATE TABLE IF NOT EXISTS export_records (
                id TEXT PRIMARY KEY,
                asset_id TEXT NOT NULL,
                format TEXT NOT NULL,
                frame_range TEXT NOT NULL,
                include_annotations BOOLEAN NOT NULL,
                output_path TEXT NOT NULL,
                exported_at TEXT NOT NULL,
                file_size_bytes INTEGER NOT NULL,
                FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE
            )",
            [],
        )?;
        
        // 创建索引
        self.db.execute(
            "CREATE INDEX IF NOT EXISTS idx_annotations_asset ON annotations(asset_id)",
            [],
        )?;
        self.db.execute(
            "CREATE INDEX IF NOT EXISTS idx_annotations_frame ON annotations(asset_id, frame_index)",
            [],
        )?;
        
        Ok(())
    }
    
    /// 创建素材
    pub fn create_asset(&mut self, asset: &Asset) -> Result<()> {
        let asset_frames_dir = self.frames_dir.join(&asset.id);
        fs::create_dir_all(&asset_frames_dir)?;
        
        self.db.execute(
            "INSERT INTO assets (
                id, source, source_url, format, frame_count, fps, 
                width, height, original_path, frame_directory, thumbnail_path,
                tags, imported_at, modified_at, view_count, last_viewed_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                asset.id,
                serde_json::to_string(&asset.source)?,
                asset.source_url,
                serde_json::to_string(&asset.format)?,
                asset.frame_count,
                asset.fps,
                asset.resolution.width,
                asset.resolution.height,
                asset.original_path.to_string_lossy(),
                asset.frame_directory.to_string_lossy(),
                asset.thumbnail_path.to_string_lossy(),
                serde_json::to_string(&asset.tags)?,
                asset.imported_at.to_rfc3339(),
                asset.modified_at.to_rfc3339(),
                asset.view_count,
                asset.last_viewed_at.map(|d| d.to_rfc3339()),
            ],
        )?;
        
        Ok(())
    }
    
    /// 获取素材
    pub fn get_asset(&mut self, id: &str) -> Result<Option<Asset>> {
        let mut stmt = self.db.prepare("SELECT * FROM assets WHERE id = ?")?;
        
        let asset = stmt.query_row([id], |row| {
            self.row_to_asset(row)
        }).optional()?;
        
        if asset.is_some() {
            self.db.execute(
                "UPDATE assets SET view_count = view_count + 1, last_viewed_at = ? WHERE id = ?",
                [Utc::now().to_rfc3339(), id.to_string()],
            )?;
        }
        
        Ok(asset)
    }
    
    /// 获取所有素材
    pub fn get_all_assets(&self) -> Result<Vec<Asset>> {
        let mut stmt = self.db.prepare("SELECT * FROM assets ORDER BY imported_at DESC")?;
        
        let assets = stmt.query_map([], |row| {
            self.row_to_asset(row)
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(assets)
    }
    
    /// 更新素材
    pub fn update_asset(&mut self, asset: &Asset) -> Result<()> {
        self.db.execute(
            "UPDATE assets SET frame_count = ?, modified_at = ? WHERE id = ?",
            params![
                asset.frame_count,
                asset.modified_at.to_rfc3339(),
                asset.id,
            ],
        )?;
        
        Ok(())
    }
    
    /// 删除素材
    pub fn delete_asset(&mut self, id: &str) -> Result<bool> {
        let deleted = self.db.execute("DELETE FROM assets WHERE id = ?", [id])?;
        
        if deleted > 0 {
            let asset_dir = self.frames_dir.join(id);
            if asset_dir.exists() {
                fs::remove_dir_all(asset_dir)?;
            }
        }
        
        Ok(deleted > 0)
    }
    
    /// 获取帧目录
    pub fn get_frame_dir(&self, asset_id: &str) -> PathBuf {
        self.frames_dir.join(asset_id)
    }
    
    /// 获取帧路径
    pub fn get_frame_path(&self, asset_id: &str, frame_index: u32) -> PathBuf {
        self.frames_dir.join(asset_id).join(format!("frame_{:06}.png", frame_index))
    }
    
    /// 创建标注
    pub fn create_annotation(&mut self, annotation: &Annotation) -> Result<()> {
        self.db.execute(
            "INSERT INTO annotations (
                id, asset_id, frame_index, annotation_type, coordinates,
                label, color, created_at, created_by, metadata
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                annotation.id,
                annotation.asset_id,
                annotation.frame_index.map(|i| i as i64),
                serde_json::to_string(&annotation.annotation_type)?,
                serde_json::to_string(&annotation.coordinates)?,
                annotation.label,
                annotation.color,
                annotation.created_at.to_rfc3339(),
                annotation.created_by,
                annotation.metadata.as_ref().map(|m| m.to_string()),
            ],
        )?;
        
        Ok(())
    }
    
    /// 获取素材的所有标注
    pub fn get_annotations_for_asset(&self, asset_id: &str) -> Result<Vec<Annotation>> {
        let mut stmt = self.db.prepare(
            "SELECT * FROM annotations WHERE asset_id = ? ORDER BY frame_index NULLS LAST, created_at"
        )?;
        
        let annotations = stmt.query_map([asset_id], |row| {
            self.row_to_annotation(row)
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(annotations)
    }
    
    /// 获取特定帧的标注
    pub fn get_annotations_for_frame(&self, asset_id: &str, frame_index: u32) -> Result<Vec<Annotation>> {
        let mut stmt = self.db.prepare(
            "SELECT * FROM annotations 
             WHERE asset_id = ? AND (frame_index = ? OR frame_index IS NULL)
             ORDER BY created_at"
        )?;
        
        let annotations = stmt.query_map(rusqlite::params![asset_id, frame_index as i64], |row| {
            self.row_to_annotation(row)
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(annotations)
    }
    
    /// 删除标注
    pub fn delete_annotation(&mut self, id: &str) -> Result<bool> {
        let deleted = self.db.execute("DELETE FROM annotations WHERE id = ?", [id])?;
        Ok(deleted > 0)
    }
    
    /// 创建导出记录
    pub fn create_export_record(&mut self, record: &ExportRecord) -> Result<()> {
        self.db.execute(
            "INSERT INTO export_records (
                id, asset_id, format, frame_range, include_annotations,
                output_path, exported_at, file_size_bytes
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                record.id,
                record.asset_id,
                serde_json::to_string(&record.format)?,
                serde_json::to_string(&record.frame_range)?,
                record.include_annotations,
                record.output_path.to_string_lossy(),
                record.exported_at.to_rfc3339(),
                record.file_size_bytes as i64,
            ],
        )?;
        
        Ok(())
    }
    
    fn row_to_asset(&self, row: &rusqlite::Row) -> rusqlite::Result<Asset> {
        Ok(Asset {
            id: row.get(0)?,
            source: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or(AssetSource::Local),
            source_url: row.get(2)?,
            format: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or(AssetFormat::Gif),
            frame_count: row.get::<_, i64>(4)? as u32,
            fps: row.get(5)?,
            resolution: Resolution::new(row.get::<_, i64>(6)? as u32, row.get::<_, i64>(7)? as u32),
            original_path: PathBuf::from(row.get::<_, String>(8)?),
            frame_directory: PathBuf::from(row.get::<_, String>(9)?),
            thumbnail_path: PathBuf::from(row.get::<_, String>(10)?),
            tags: serde_json::from_str(&row.get::<_, String>(11)?).unwrap_or_default(),
            imported_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(12)?)
                .unwrap().with_timezone(&Utc),
            modified_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(13)?)
                .unwrap().with_timezone(&Utc),
            view_count: row.get::<_, i64>(14)? as u32,
            last_viewed_at: row.get::<_, Option<String>>(15)?.map(|s| {
                DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&Utc)
            }),
        })
    }
    
    fn row_to_annotation(&self, row: &rusqlite::Row) -> rusqlite::Result<Annotation> {
        Ok(Annotation {
            id: row.get(0)?,
            asset_id: row.get(1)?,
            frame_index: row.get::<_, Option<i64>>(2)?.map(|i| i as u32),
            annotation_type: serde_json::from_str(&row.get::<_, String>(3)?)
                .unwrap_or(AnnotationType::Rect),
            coordinates: serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
            label: row.get(5)?,
            color: row.get(6)?,
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                .unwrap().with_timezone(&Utc),
            created_by: row.get(8)?,
            metadata: row.get::<_, Option<String>>(9)?.map(|s| serde_json::from_str(&s).unwrap()),
        })
    }

    /// 获取临时目录
    pub fn get_temp_dir(&self) -> PathBuf {
        let temp_dir = self.data_dir.join("temp");
        std::fs::create_dir_all(&temp_dir).ok();
        temp_dir
    }
}