# Animation Asset Manager (AAM) - 动画素材管理器

## 项目概述

跨平台多媒体素材管理工具，专为动画师和特效师设计。支持GIF逐帧预览、标注、H264导出，并与Maya/Blender无缝集成。

## 核心设计哲学

### Deep Modules 架构
每个模块提供简洁接口，隐藏复杂实现：
- **DecoderService**: `decode(file) → Frame[]` - 隐藏GIF/WebM解析复杂性
- **EncoderService**: `encode(frames, options) → Blob` - 隐藏FFmpeg编码复杂性  
- **DCCIntegration**: `exportToMaya(asset)` / `exportToBlender(asset)` - 隐藏DCC软件API差异
- **AnnotationEngine**: `draw(ctx, annotation)` - 隐藏Canvas绘制细节

### 信息隐藏
- 存储层细节不泄露到UI层
- 视频编解码参数集中在EncoderService内部
- Maya/Blender的Python API差异由适配器层消化

### 错误消除
- 素材自动转码为统一内部格式
- 帧索引越界自动裁剪而非报错
- 缺失标注渲染时静默跳过而非崩溃

## 技术栈决策

| 层级 | 技术 | 理由 |
|------|------|------|
| 桌面框架 | **Tauri v2** | 比Electron轻量80%，原生FFmpeg集成，Rust性能 |
| 前端 | React + TypeScript + Tailwind | 组件化开发，类型安全 |
| 多媒体 | FFmpeg (Rust绑定) + wasm | 跨平台视频处理 |
| 存储 | SQLite (via Rusqlite) + 文件系统 | 元数据结构化，大文件独立存储 |
| Maya集成 | Python插件 | 标准mel/python命令通道 |
| Blender集成 | Python插件 | bpy API原生支持 |

## 数据模型 (Deep Module: StorageManager)

```typescript
// 核心实体 - 完全隐藏存储细节
interface Asset {
  id: string;                    // 业务标识
  source: 'sakugabooru' | 'local' | 'url';
  
  // 媒体信息 (内部统一格式)
  format: 'gif' | 'webm' | 'mp4' | 'image_sequence';
  frameCount: number;
  fps: number;
  resolution: { width: number; height: number };
  
  // 文件路径 (相对于应用数据目录)
  originalPath: string;          // 原始文件
  frameDirectory: string;        // 解帧输出目录
  thumbnailPath: string;       // 缩略图
  
  // 元数据
  tags: Tag[];
  importedAt: Date;
  
  // 关联数据
  annotations: Annotation[];
  exports: ExportRecord[];
}

interface Frame {
  assetId: string;
  index: number;
  timestamp: number;
  fileName: string;              // 如 "frame_0042.png"
  hasAnnotation: boolean;
}

interface Annotation {
  id: string;
  assetId: string;
  frameIndex: number | 'global';  // global表示跨所有帧
  type: 'rect' | 'circle' | 'arrow' | 'text' | 'stroke';
  coordinates: NormalizedCoords;  // 0-1相对坐标，适配不同分辨率
  label?: string;
  color: string;
  createdAt: Date;
  metadata?: Record<string, any>; // 扩展数据
}

// DCC导出配置 (通用抽象，隐藏Maya/Blender差异)
interface ExportConfig {
  format: 'image_sequence' | 'video' | 'maya_playblast' | 'blender_viewport';
  frameRange: { start: number; end: number } | 'all';
  resolution: 'original' | 'half' | 'quarter' | CustomResolution;
  includeAnnotations: boolean;
  destinationPath: string;
}
```

## 模块架构

```
src-tauri/           (Rust后端 - Deep Modules)
  src/
    main.rs          # 入口
    commands/        # Tauri命令层 (薄层，无业务逻辑)
      asset.rs
      decoder.rs
      encoder.rs
      annotation.rs
    services/        # 业务逻辑层
      asset_service.rs       # 素材CRUD + 查询
      decoder_service.rs     # 多媒体解码
      encoder_service.rs     # H264编码
      storage_service.rs     # 文件+数据库管理
      annotation_service.rs  # 标注CRUD
    dcc/             # DCC集成层
      maya_bridge.rs         # Maya通信
      blender_bridge.rs      # Blender通信
    models/          # 数据结构
    ffmpeg/          # FFmpeg封装

src/                 (React前端)
  components/
    viewer/          # 逐帧预览器核心
    timeline/        # 时间轴控制
    annotation/      # 标注工具栏+画布
    browser/         # 素材浏览器
  hooks/
    useAsset.ts
    useFrames.ts
    useAnnotations.ts
  services/
    api.ts           # Tauri命令调用

plugins/             (DCC插件)
  maya/
    AAMConnector.py  # Maya端插件
  blender/
    aam_connector.py # Blender端插件
```

## 关键实现策略

### 1. 逐帧预览 (Viewer Module)
- 预加载：当前帧±5帧在内存，其余按需从磁盘读取
- 缓存策略：LRU缓存最近访问的50帧
- 解码：GIF用`gif` crate，WebM/MP4用FFmpeg解帧到PNG序列

### 2. H264导出 (Encoder Module)
- 输入：标注合成后的帧序列
- 处理：FFmpeg H264编码，YUV420p像素格式，CRF 18-23质量
- 输出：MP4文件，支持Maya/Blender直接读取

### 3. Maya/Blender集成 (DCC Bridge)
**设计原则**: 插件最小化，通信标准化

```rust
// Rust端 (Tauri)
pub trait DCCBridge {
    fn is_running(&self) -> bool;
    fn export_frames(&self, path: &str, frames: &[Frame]) -> Result<()>;
    fn import_as_plane(&self, asset: &Asset) -> Result<()>;
}
```

```python
# Maya端插件 (极简，只接收命令)
import maya.cmds as cmds
import maya.utils as utils
import json
import os

def aam_import_image_sequence(path, frame_range=None):
    """从AAM导入图像序列为imagePlane"""
    if not os.path.exists(path):
        cmds.warning(f"Path not found: {path}")
        return
    
    # 创建imagePlane
    plane = cmds.imagePlane(fileName=path)
    
    # 设置帧范围
    if frame_range:
        cmds.setAttr(f"{plane[0]}.frameOffset", frame_range[0])
    
    return plane

def aam_import_annotations(asset_data):
    """导入标注为locator或曲线"""
    annotations = json.loads(asset_data)
    created = []
    
    for ann in annotations:
        if ann['type'] == 'rect':
            # 创建方框曲线
            loc = cmds.spaceLocator(name=f"aam_ann_{ann['id']}")[0]
            cmds.setAttr(f"{loc}.translateX", ann['coords']['x'])
            cmds.setAttr(f"{loc}.translateY", ann['coords']['y'])
            created.append(loc)
    
    return created
```

### 4. 错误处理策略
- **定义错误为不存在**：帧索引越界时返回首尾帧而非抛出
- **异常聚合**：多媒体错误统一映射为`MediaError`类型
- **降级处理**：FFmpeg失败时回退到逐帧PNG输出

## UI设计 (Bento Paradigm)

- **左侧栏**：素材浏览器 (Bento网格)
- **主区域**：逐帧预览器 + 标注画布叠加
- **底部**：时间轴 + 播放控制
- **右侧**：属性面板 + 标注列表

**禁止模式**：
- 无蓝紫渐变，使用中性灰+单一强调色
- 无3列等宽卡片，使用Bento不等高布局
- 无通用卡片，使用边框分隔

## 开发阶段

1. **核心基础设施** (MVP)
   - Tauri项目搭建
   - SQLite数据库
   - 基础素材CRUD

2. **逐帧预览**
   - GIF解帧
   - 帧缓存系统
   - 播放控制

3. **标注系统**
   - Canvas绘制
   - 帧关联存储

4. **导出功能**
   - FFmpeg集成
   - H264编码

5. **DCC集成**
   - Maya插件
   - Blender插件

## 质量保证

- [ ] Undo支持：所有操作可撤销
- [ ] 场景验证：DCC连接前检查
- [ ] 性能：1000帧素材流畅播放
- [ ] 跨平台：Windows 10+ / Ubuntu 20.04+ 测试通过
- [ ] 错误处理：用户友好错误消息

---
*Design Philosophy: Deep Modules, Information Hiding, Define Errors Out*
