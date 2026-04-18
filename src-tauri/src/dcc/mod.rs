//! DCC导出配置 - 简化模块
//! 
//! 不再开发Maya/Blender插件，只提供简单的导出配置和可选的辅助脚本生成

use std::path::Path;
use serde::{Serialize, Deserialize};

/// 导出目标配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExportConfig {
    /// 导出格式
    pub format: ExportFormat,
    /// 帧范围
    pub frame_range: FrameRange,
    /// 包含标注叠加
    pub include_annotations: bool,
    /// 输出路径
    pub output_path: String,
}

/// 导出格式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    /// PNG序列帧 - 最高质量，DCC软件通用
    PngSequence,
    /// H264视频 - 体积小，适合预览
    H264,
    /// ProRes - 专业后期流程
    ProRes,
    /// GIF - 适合网页展示
    Gif,
    /// WebM - 现代浏览器支持
    WebM,
}

impl Default for ExportFormat {
    fn default() -> Self {
        ExportFormat::PngSequence
    }
}

/// 帧范围
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FrameRange {
    All,
    Custom { start: u32, end: u32 },
}

impl Default for FrameRange {
    fn default() -> Self {
        FrameRange::All
    }
}

impl FrameRange {
    /// 获取实际起始帧 (越界时自动裁剪)
    pub fn start(&self, total_frames: u32) -> u32 {
        match self {
            FrameRange::All => 1,
            FrameRange::Custom { start, .. } => (*start).min(total_frames).max(1),
        }
    }
    
    /// 获取实际结束帧
    pub fn end(&self, total_frames: u32) -> u32 {
        match self {
            FrameRange::All => total_frames,
            FrameRange::Custom { end, .. } => (*end).min(total_frames).max(1),
        }
    }
    
    /// 计算帧数
    pub fn count(&self, total_frames: u32) -> u32 {
        let start = self.start(total_frames);
        let end = self.end(total_frames);
        if end >= start {
            end - start + 1
        } else {
            1
        }
    }
}

/// 生成Maya导入辅助脚本（可选，用户手动运行）
/// 
/// 这个脚本会创建imagePlane并设置好图像序列
pub fn generate_maya_import_script(sequence_path: &Path, fps: f32) -> String {
    let pattern = sequence_path.to_string_lossy().replace("\\", "/");
    format!(r#"# AAM导入脚本 - 手动粘贴到Maya Python中运行
import maya.cmds as cmds

# 创建imagePlane
plane = cmds.imagePlane(fileName='{}')

# 设置序列帧
if cmds.objExists(plane[0]):
    cmds.setAttr("{{}}.useFrameExtension".format(plane[0]), 1)
    cmds.setAttr("{{}}.frameOffset".format(plane[0]), 1)
    cmds.setAttr("{{}}.frameCache".format(plane[0]), 0)  # 不缓存，实时读取
    
    # 重命名
    cmds.rename(plane[0], "aam_sequence_plane")
    print("已创建imagePlane: aam_sequence_plane")
    print("帧率: {}fps".format({}))
"#, pattern, fps)
}

/// 生成Blender导入辅助脚本（可选，用户手动运行）
pub fn generate_blender_import_script(sequence_path: &Path, frame_count: u32) -> String {
    let pattern = sequence_path.to_string_lossy().replace("\\", "/");
    format!(r#"# AAM导入脚本 - 在Blender Scripting标签页中运行
import bpy

# 加载图像序列
img = bpy.data.images.load('{}')
img.source = 'SEQUENCE'

# 创建平面
bpy.ops.mesh.primitive_plane_add(size=1, location=(0, 0, 0))
plane = bpy.context.active_object
plane.name = "aam_sequence_plane"

# 创建材质
mat = bpy.data.materials.new(name="aam_sequence_mat")
mat.use_nodes = True
plane.data.materials.append(mat)

# 设置节点
nodes = mat.node_tree.nodes
links = mat.node_tree.links

# 清除默认节点
for node in list(nodes):
    nodes.remove(node)

# 创建图像纹理节点
tex_node = nodes.new('ShaderNodeTexImage')
tex_node.image = img
tex_node.image_user.frame_start = 1
tex_node.image_user.frame_duration = {}

# 创建输出节点
output = nodes.new('ShaderNodeOutputMaterial')
output.location = (300, 0)

# 连接
links.new(tex_node.outputs['Color'], output.inputs['Base Color'])

print("已创建图像平面: aam_sequence_plane")
print("总帧数: {}".format({}))
"#, pattern, frame_count, frame_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_frame_range_bounds() {
        // Define Errors Out: 越界自动裁剪
        let range = FrameRange::Custom { start: 0, end: 1000 };
        assert_eq!(range.end(100), 100);  // 自动裁剪到实际帧数
        assert_eq!(range.start(100), 1);  // 最小为1
        
        let range = FrameRange::Custom { start: 50, end: 60 };
        assert_eq!(range.count(100), 11); // 50到60共11帧
    }
    
    #[test]
    fn test_maya_script_generation() {
        let script = generate_maya_import_script(
            std::path::Path::new("/test/frame_000001.png"),
            30.0
        );
        assert!(script.contains("imagePlane"));
        assert!(script.contains("useFrameExtension"));
    }
}