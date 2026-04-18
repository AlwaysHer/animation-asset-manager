// 类型定义 - 与 Rust 后端对应

// 素材来源
export type AssetSource = 'sakugabooru' | 'local' | 'url';

// 素材格式
export type AssetFormat = 'gif' | 'webm' | 'mp4' | 'mov' | 'image_sequence' | 'png' | 'jpg';

// 分辨率
export interface Resolution {
  width: number;
  height: number;
}

// 素材实体
export interface Asset {
  id: string;
  source: AssetSource;
  source_url?: string;
  format: AssetFormat;
  frame_count: number;
  fps: number;
  resolution: Resolution;
  original_path: string;
  frame_directory: string;
  thumbnail_path: string;
  tags: Tag[];
  imported_at: string;
  modified_at: string;
  view_count: number;
  last_viewed_at?: string;
}

// 标签
export interface Tag {
  name: string;
  category: TagCategory;
  confidence?: number;
}

export type TagCategory = 'character' | 'copyright' | 'artist' | 'general' | 'meta';

// 帧信息
export interface Frame {
  asset_id: string;
  index: number;
  timestamp_ms: number;
  filename: string;
  has_annotation: boolean;
}

// 标注类型
export type AnnotationType = 'rect' | 'circle' | 'arrow' | 'text' | 'stroke';

// 标注实体
export interface Annotation {
  id: string;
  asset_id: string;
  frame_index?: number;  // undefined = 全局标注
  annotation_type: AnnotationType;
  coordinates: AnnotationCoords;
  label?: string;
  color: string;  // hex: #RRGGBB
  created_at: string;
  created_by: string;
  metadata?: Record<string, unknown>;
}

// 标注坐标
export type AnnotationCoords =
  | { type: 'rect'; x: number; y: number; width: number; height: number }
  | { type: 'circle'; cx: number; cy: number; r: number }
  | { type: 'arrow'; x1: number; y1: number; x2: number; y2: number }
  | { type: 'text'; x: number; y: number }
  | { type: 'stroke'; points: [number, number][] };

// 导出格式
export type ExportFormat = 
  | 'png_sequence' 
  | 'h264' 
  | 'pro_res' 
  | 'gif' 
  | 'webm' 
  | 'maya_playblast' 
  | 'blender_viewport';

// 帧范围
export type FrameRange =
  | { type: 'all' }
  | { type: 'custom'; start: number; end: number };

// 导出配置
export interface ExportConfig {
  format: ExportFormat;
  frame_range: FrameRange;
  include_annotations: boolean;
  output_path: string;
}

// 导出记录
export interface ExportRecord {
  id: string;
  asset_id: string;
  format: ExportFormat;
  frame_range: FrameRange;
  include_annotations: boolean;
  output_path: string;
  exported_at: string;
  file_size_bytes: number;
}

// 素材过滤器
export interface AssetFilter {
  tags?: string[];
  format?: AssetFormat;
  source?: AssetSource;
  has_annotations?: boolean;
  imported_after?: string;
  imported_before?: string;
  search_text?: string;
}

// 键盘事件处理
export interface KeyBinding {
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  action: () => void;
}
