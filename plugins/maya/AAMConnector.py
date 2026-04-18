#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
AAM Connector for Autodesk Maya
动画素材管理器 - Maya端插件

设计原则:
- 最小化: 只接收命令，不处理复杂逻辑
- 标准化: 使用JSON参数通信
- 向后兼容: 支持Maya 2022+ (Python 3)

安装:
1. 将本文件复制到 Maya的脚本目录:
   - Windows: %USERPROFILE%\Documents\maya\scripts
   - Linux: ~/maya/scripts
   - macOS: ~/Library/Preferences/Autodesk/maya/scripts

2. 在Maya中执行:
   import AAMConnector
   AAMConnector.show_ui()
"""

import os
import sys
import json
import tempfile
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass

try:
    import maya.cmds as cmds
    import maya.utils as utils
    import maya.api.OpenMaya as om2
    HAS_MAYA = True
except ImportError:
    HAS_MAYA = False
    print("Warning: Maya modules not available. Running in test mode.")


def require_maya(func):
    """装饰器: 确保Maya可用"""
    def wrapper(*args, **kwargs):
        if not HAS_MAYA:
            raise RuntimeError("Maya API not available")
        return func(*args, **kwargs)
    return wrapper


# =============================================================================
# 数据模型
# =============================================================================

@dataclass
class AAMAssetInfo:
    """AAM素材信息"""
    asset_id: str
    frame_count: int
    fps: float
    width: int
    height: int
    frame_pattern: str  # 图像序列路径模式


@dataclass
class AAMAnnotation:
    """AAM标注信息"""
    annotation_id: str
    frame_index: Optional[int]  # None表示全局标注
    ann_type: str  # 'rect', 'circle', 'arrow', 'text'
    x: float  # 归一化坐标 0-1
    y: float
    width: float = 0.0
    height: float = 0.0
    label: str = ""
    color: Tuple[float, float, float] = (1.0, 1.0, 0.0)  # RGB


# =============================================================================
# 核心功能
# =============================================================================

class AAMConnector:
    """AAM连接器主类"""
    
    VERSION = "1.0.0"
    
    def __init__(self):
        self.imported_assets = {}  # asset_id -> node_name mapping
    
    # -------------------------------------------------------------------------
    # 导入功能
    # -------------------------------------------------------------------------
    
    @require_maya
    def import_image_sequence(self, asset_info: AAMAssetInfo, 
                             create_camera: bool = True) -> str:
        """
        导入图像序列为imagePlane
        
        Args:
            asset_info: 素材信息
            create_camera: 是否创建匹配的摄像机
            
        Returns:
            imagePlane节点名称
        """
        # 创建imagePlane
        plane_nodes = cmds.imagePlane(fileName=asset_info.frame_pattern)
        plane_shape = plane_nodes[0] if isinstance(plane_nodes, list) else plane_nodes
        plane_transform = cmds.listRelatives(plane_shape, parent=True)[0]
        
        # 配置imagePlane
        cmds.setAttr(f"{plane_shape}.useFrameExtension", 1)
        cmds.setAttr(f"{plane_shape}.frameOffset", 1)
        
        # 设置尺寸 (转换为Maya单位)
        width_maya = asset_info.width / 100.0
        height_maya = asset_info.height / 100.0
        cmds.setAttr(f"{plane_transform}.scaleX", width_maya)
        cmds.setAttr(f"{plane_transform}.scaleY", height_maya)
        
        # 重命名
        new_name = f"aam_{asset_info.asset_id}_plane"
        plane_transform = cmds.rename(plane_transform, new_name)
        
        # 记录
        self.imported_assets[asset_info.asset_id] = plane_transform
        
        # 创建匹配摄像机
        if create_camera:
            self.create_matching_camera(asset_info)
        
        # 设置播放范围
        cmds.playbackOptions(min=1, max=asset_info.frame_count)
        
        return plane_transform
    
    @require_maya
    def create_matching_camera(self, asset_info: AAMAssetInfo) -> str:
        """
        创建匹配素材视角的摄像机
        
        Args:
            asset_info: 素材信息
            
        Returns:
            摄像机节点名称
        """
        # 创建摄像机
        cam_transform, cam_shape = cmds.camera()
        
        # 设置焦距
        cmds.setAttr(f"{cam_shape}.focalLength", 35)
        
        # 计算并设置光圈以匹配素材宽高比
        aspect = asset_info.width / asset_info.height
        
        # 设置渲染分辨率
        cmds.setAttr("defaultResolution.width", asset_info.width)
        cmds.setAttr("defaultResolution.height", asset_info.height)
        cmds.setAttr("defaultResolution.deviceAspectRatio", aspect)
        
        # 重命名
        new_name = f"aam_{asset_info.asset_id}_cam"
        cam_transform = cmds.rename(cam_transform, new_name)
        
        # 定位摄像机以匹配平面
        # 计算距离使画面填满视图
        plane_width = asset_info.width / 100.0
        fov = 35  # 焦距35mm的FOV约38.5度
        distance = (plane_width / 2) / (fov * 3.14159 / 180)
        
        cmds.setAttr(f"{cam_transform}.translateZ", distance * 100)
        cmds.setAttr(f"{cam_transform}.rotateX", -90)  # 指向平面
        
        return cam_transform
    
    @require_maya
    def import_annotations(self, asset_info: AAMAssetInfo, 
                           annotations: List[AAMAnnotation]) -> List[str]:
        """
        导入标注为Maya locator/曲线
        
        Args:
            asset_info: 素材信息
            annotations: 标注列表
            
        Returns:
            创建的节点名称列表
        """
        created = []
        
        # 创建标注组
        group_name = f"aam_{asset_info.asset_id}_annotations"
        if cmds.objExists(group_name):
            cmds.delete(group_name)
        
        group = cmds.group(empty=True, name=group_name)
        
        for ann in annotations:
            nodes = self._create_annotation_node(asset_info, ann)
            if nodes:
                for node in nodes:
                    cmds.parent(node, group)
                created.extend(nodes)
        
        return created
    
    def _create_annotation_node(self, asset_info: AAMAssetInfo, 
                                ann: AAMAnnotation) -> List[str]:
        """创建单个标注节点"""
        nodes = []
        
        # 像素坐标
        px = ann.x * asset_info.width
        py = (1 - ann.y) * asset_info.height  # Maya Y轴向上，图像Y轴向下
        
        if ann.ann_type == 'rect':
            # 创建方框 - 使用4条曲线
            w = ann.width * asset_info.width / 100.0
            h = ann.height * asset_info.height / 100.0
            px_maya = px / 100.0
            py_maya = py / 100.0
            
            corners = [
                (px_maya - w/2, py_maya - h/2, 0),
                (px_maya + w/2, py_maya - h/2, 0),
                (px_maya + w/2, py_maya + h/2, 0),
                (px_maya - w/2, py_maya + h/2, 0),
                (px_maya - w/2, py_maya - h/2, 0),
            ]
            
            curve = cmds.curve(degree=1, point=corners)
            curve = cmds.rename(curve, f"aam_ann_{ann.annotation_id}")
            
            # 设置颜色 (黄色)
            cmds.setAttr(f"{curve}.overrideEnabled", 1)
            cmds.setAttr(f"{curve}.overrideColor", 17)
            
            nodes.append(curve)
            
            # 如果有标签，创建文本
            if ann.label:
                text = cmds.textCurves(text=ann.label)[0]
                text = cmds.rename(text, f"aam_ann_{ann.annotation_id}_label")
                cmds.setAttr(f"{text}.translate", px_maya, py_maya + h/2 + 0.5, 0, type="double3")
                cmds.setAttr(f"{text}.scale", 0.3, 0.3, 0.3, type="double3")
                nodes.append(text)
                
        elif ann.ann_type == 'circle':
            # 创建圆形
            locator = cmds.spaceLocator(name=f"aam_ann_{ann.annotation_id}")[0]
            cmds.setAttr(f"{locator}.translate", px/100.0, py/100.0, 0, type="double3")
            
            # 设置颜色
            cmds.setAttr(f"{locator}.overrideEnabled", 1)
            cmds.setAttr(f"{locator}.overrideColor", 17)
            
            nodes.append(locator)
            
        elif ann.ann_type == 'arrow':
            # 创建箭头曲线
            # 简化为两点线
            start = (px/100.0, py/100.0, 0)
            end = (px/100.0 + 2, py/100.0 + 2, 0)  # 简化箭头
            
            curve = cmds.curve(degree=1, point=[start, end])
            curve = cmds.rename(curve, f"aam_ann_{ann.annotation_id}")
            
            cmds.setAttr(f"{curve}.overrideEnabled", 1)
            cmds.setAttr(f"{curve}.overrideColor", 17)
            
            nodes.append(curve)
        
        return nodes
    
    # -------------------------------------------------------------------------
    # 导出功能
    # -------------------------------------------------------------------------
    
    @require_maya
    def export_viewport_sequence(self, output_path: str, 
                                 frame_range: Optional[Tuple[int, int]] = None,
                                 resolution: Optional[Tuple[int, int]] = None) -> str:
        """
        导出视口播放为图像序列
        
        Args:
            output_path: 输出路径模式 (如 "/path/frame_%04d.png")
            frame_range: (开始帧, 结束帧)，None使用当前播放范围
            resolution: (宽, 高)，None使用当前渲染设置
            
        Returns:
            输出路径
        """
        # 保存当前渲染设置
        orig_format = cmds.getAttr("defaultRenderGlobals.imageFormat")
        orig_width = cmds.getAttr("defaultResolution.width")
        orig_height = cmds.getAttr("defaultResolution.height")
        
        # 设置输出格式为PNG
        cmds.setAttr("defaultRenderGlobals.imageFormat", 32)  # PNG
        
        # 设置分辨率
        if resolution:
            cmds.setAttr("defaultResolution.width", resolution[0])
            cmds.setAttr("defaultResolution.height", resolution[1])
        
        # 确定帧范围
        if frame_range:
            start, end = frame_range
        else:
            start = int(cmds.playbackOptions(q=True, min=True))
            end = int(cmds.playbackOptions(q=True, max=True))
        
        # 使用playblast导出
        cmds.playblast(
            filename=output_path.replace("%04d", "").rstrip("."),
            format="image",
            sequenceTime=True,
            clearCache=True,
            viewer=False,
            showOrnaments=False,
            framePadding=4,
            percent=100,
            compression="png",
            quality=100,
            startTime=start,
            endTime=end,
        )
        
        # 恢复设置
        cmds.setAttr("defaultRenderGlobals.imageFormat", orig_format)
        cmds.setAttr("defaultResolution.width", orig_width)
        cmds.setAttr("defaultResolution.height", orig_height)
        
        return output_path


# =============================================================================
# UI (Maya内嵌界面)
# =============================================================================

@require_maya
def show_ui():
    """显示AAM连接器UI"""
    
    window_name = "AAMConnectorWindow"
    
    if cmds.window(window_name, exists=True):
        cmds.deleteUI(window_name)
    
    window = cmds.window(
        window_name,
        title="AAM Connector",
        widthHeight=(300, 200),
        sizeable=False,
    )
    
    cmds.columnLayout(adjustable=True)
    
    cmds.text(label="Animation Asset Manager", font="boldLabelFont")
    cmds.separator(height=10)
    
    cmds.text(label="Status: Connected", backgroundColor=(0.2, 0.6, 0.2))
    cmds.separator(height=10)
    
    cmds.button(
        label="Import from AAM",
        command=lambda *args: _show_import_dialog()
    )
    
    cmds.button(
        label="Export to AAM",
        command=lambda *args: _show_export_dialog()
    )
    
    cmds.button(
        label="Close",
        command=lambda *args: cmds.deleteUI(window_name)
    )
    
    cmds.showWindow(window)


def _show_import_dialog():
    """显示导入对话框"""
    result = cmds.fileDialog2(
        fileMode=1,
        caption="Select AAM Asset JSON",
        fileFilter="JSON files (*.json)",
    )
    
    if result:
        with open(result[0], 'r') as f:
            data = json.load(f)
        
        connector = AAMConnector()
        asset_info = AAMAssetInfo(
            asset_id=data['id'],
            frame_count=data['frame_count'],
            fps=data['fps'],
            width=data['width'],
            height=data['height'],
            frame_pattern=data['frame_pattern'],
        )
        
        node = connector.import_image_sequence(asset_info)
        cmds.confirmDialog(message=f"Imported: {node}")


def _show_export_dialog():
    """显示导出对话框"""
    result = cmds.fileDialog2(
        fileMode=0,
        caption="Export Viewport Sequence",
        fileFilter="PNG sequence (*.png)",
    )
    
    if result:
        connector = AAMConnector()
        output = connector.export_viewport_sequence(result[0])
        cmds.confirmDialog(message=f"Exported to: {output}")


# =============================================================================
# 命令行接口 (用于Rust调用)
# =============================================================================

def main():
    """命令行入口"""
    import argparse
    
    parser = argparse.ArgumentParser(description="AAM Connector for Maya")
    parser.add_argument("--command", required=True, 
                       choices=["import", "export", "create_camera", "import_annotations"])
    parser.add_argument("--data", required=True, help="JSON data file path")
    
    args = parser.parse_args()
    
    # 读取数据
    with open(args.data, 'r') as f:
        data = json.load(f)
    
    connector = AAMConnector()
    
    if args.command == "import":
        asset_info = AAMAssetInfo(**data['asset'])
        node = connector.import_image_sequence(asset_info, data.get('create_camera', True))
        print(json.dumps({"success": True, "node": node}))
        
    elif args.command == "export":
        frame_range = data.get('frame_range')
        resolution = data.get('resolution')
        output = connector.export_viewport_sequence(
            data['output_path'],
            tuple(frame_range) if frame_range else None,
            tuple(resolution) if resolution else None,
        )
        print(json.dumps({"success": True, "output": output}))
        
    elif args.command == "create_camera":
        asset_info = AAMAssetInfo(**data['asset'])
        node = connector.create_matching_camera(asset_info)
        print(json.dumps({"success": True, "camera": node}))
        
    elif args.command == "import_annotations":
        asset_info = AAMAssetInfo(**data['asset'])
        annotations = [AAMAnnotation(**a) for a in data['annotations']]
        nodes = connector.import_annotations(asset_info, annotations)
        print(json.dumps({"success": True, "nodes": nodes}))


if __name__ == "__main__":
    if HAS_MAYA and len(sys.argv) > 1:
        main()
    elif HAS_MAYA:
        show_ui()
    else:
        print("Maya API not available. Plugin loaded for reference only.")
