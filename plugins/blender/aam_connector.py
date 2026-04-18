#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
AAM Connector for Blender
动画素材管理器 - Blender端插件

设计原则:
- 最小化: 只接收命令，不处理复杂逻辑
- 标准化: 使用JSON参数通信
- 向后兼容: 支持Blender 3.0+

安装:
1. 打开Blender -> Edit -> Preferences -> Add-ons
2. 点击 "Install..."
3. 选择本文件
4. 启用插件 "Animation: AAM Connector"

使用:
- 3D Viewport侧边栏 (N键) -> AAM标签
"""

bl_info = {
    "name": "AAM Connector",
    "author": "Animation Asset Manager",
    "version": (1, 0, 0),
    "blender": (3, 0, 0),
    "location": "View3D > Sidebar > AAM",
    "description": "Connect with Animation Asset Manager for frame-by-frame reference",
    "category": "Animation",
}

import os
import json
import bpy
import bmesh
from bpy.props import StringProperty, BoolProperty, IntProperty, FloatProperty
from bpy.types import Panel, Operator, PropertyGroup
from mathutils import Vector
from dataclasses import dataclass
from typing import List, Optional, Tuple, Dict


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
    frame_pattern: str
    
    @classmethod
    def from_dict(cls, data: Dict) -> "AAMAssetInfo":
        return cls(
            asset_id=data['asset_id'],
            frame_count=data['frame_count'],
            fps=data['fps'],
            width=data['width'],
            height=data['height'],
            frame_pattern=data['frame_pattern'],
        )


@dataclass  
class AAMAnnotation:
    """AAM标注信息"""
    annotation_id: str
    frame_index: Optional[int]
    ann_type: str
    x: float
    y: float
    width: float = 0.0
    height: float = 0.0
    label: str = ""
    color: Tuple[float, float, float] = (1.0, 1.0, 0.0)
    
    @classmethod
    def from_dict(cls, data: Dict) -> "AAMAnnotation":
        return cls(
            annotation_id=data['annotation_id'],
            frame_index=data.get('frame_index'),
            ann_type=data['ann_type'],
            x=data['x'],
            y=data['y'],
            width=data.get('width', 0.0),
            height=data.get('height', 0.0),
            label=data.get('label', ''),
            color=tuple(data.get('color', [1.0, 1.0, 0.0])),
        )


# =============================================================================
# 核心功能
# =============================================================================

class AAMConnector:
    """AAM连接器主类"""
    
    def __init__(self):
        self.imported_assets: Dict[str, str] = {}  # asset_id -> object_name
    
    # -------------------------------------------------------------------------
    # 导入功能
    # -------------------------------------------------------------------------
    
    def import_image_sequence(self, asset_info: AAMAssetInfo, 
                           context: bpy.types.Context,
                           create_camera: bool = True) -> bpy.types.Object:
        """
        导入图像序列为平面对象
        
        Args:
            asset_info: 素材信息
            context: Blender上下文
            create_camera: 是否创建匹配摄像机
            
        Returns:
            平面对象
        """
        # 加载图像序列
        if not os.path.exists(asset_info.frame_pattern.replace("%06d", "000001")):
            # 尝试查找第一个帧
            base_dir = os.path.dirname(asset_info.frame_pattern)
            if os.path.isdir(base_dir):
                files = sorted([f for f in os.listdir(base_dir) if f.endswith('.png')])
                if files:
                    first_frame = os.path.join(base_dir, files[0])
                    asset_info.frame_pattern = os.path.join(base_dir, "frame_%06d.png")
        
        img = bpy.data.images.load(asset_info.frame_pattern, check_existing=False)
        img.source = 'SEQUENCE'
        img.name = f"aam_{asset_info.asset_id}_seq"
        
        # 创建平面
        bpy.ops.mesh.primitive_plane_add(size=1, location=(0, 0, 0))
        plane = context.active_object
        plane.name = f"aam_{asset_info.asset_id}_plane"
        
        # 计算尺寸 (保持宽高比)
        aspect = asset_info.width / asset_info.height
        plane.scale = (aspect, 1.0, 1.0)
        
        # 创建材质
        mat = bpy.data.materials.new(name=f"aam_{asset_info.asset_id}_mat")
        mat.use_nodes = True
        mat.use_backface_culling = False
        mat.blend_method = 'BLEND'
        mat.shadow_method = 'NONE'
        
        # 设置节点
        nodes = mat.node_tree.nodes
        links = mat.node_tree.links
        
        # 清除默认节点
        for node in nodes:
            nodes.remove(node)
        
        # 创建图像纹理节点
        tex_node = nodes.new('ShaderNodeTexImage')
        tex_node.image = img
        tex_node.location = (-400, 0)
        
        # 设置图像用户
        tex_node.image_user.frame_start = 1
        tex_node.image_user.frame_duration = asset_info.frame_count
        tex_node.image_user.use_cyclic = False
        
        # 创建材质输出节点
        output = nodes.new('ShaderNodeOutputMaterial')
        output.location = (0, 0)
        
        # 创建发射节点 (让平面自发光，不受场景光照影响)
        emission = nodes.new('ShaderNodeEmission')
        emission.location = (-200, 0)
        emission.inputs['Strength'].default_value = 1.0
        
        # 连接节点
        links.new(tex_node.outputs['Color'], emission.inputs['Color'])
        links.new(tex_node.outputs['Alpha'], emission.inputs['Strength'])
        links.new(emission.outputs['Emission'], output.inputs['Surface'])
        
        # 分配材质
        if plane.data.materials:
            plane.data.materials[0] = mat
        else:
            plane.data.materials.append(mat)
        
        # 记录
        self.imported_assets[asset_info.asset_id] = plane.name
        
        # 创建匹配摄像机
        if create_camera:
            self.create_matching_camera(asset_info, context)
        
        # 设置场景帧范围
        context.scene.frame_start = 1
        context.scene.frame_end = asset_info.frame_count
        context.scene.render.fps = int(asset_info.fps)
        context.scene.render.fps_base = asset_info.fps / int(asset_info.fps)
        
        return plane
    
    def create_matching_camera(self, asset_info: AAMAssetInfo,
                            context: bpy.types.Context) -> bpy.types.Object:
        """
        创建匹配素材视角的摄像机
        
        Args:
            asset_info: 素材信息
            context: Blender上下文
            
        Returns:
            摄像机对象
        """
        # 创建摄像机数据
        cam_data = bpy.data.cameras.new(name=f"aam_{asset_info.asset_id}_cam")
        
        # 设置分辨率
        context.scene.render.resolution_x = asset_info.width
        context.scene.render.resolution_y = asset_info.height
        context.scene.render.resolution_percentage = 100
        
        # 计算焦距以匹配平面
        # 假设传感器宽度36mm，计算匹配视角的焦距
        aspect = asset_info.width / asset_info.height
        sensor_width = 36.0
        sensor_height = sensor_width / aspect
        
        cam_data.sensor_width = sensor_width
        cam_data.sensor_height = sensor_height
        
        # 焦距35mm是良好的默认值
        cam_data.lens = 35.0
        
        # 创建摄像机对象
        cam_obj = bpy.data.objects.new(f"aam_{asset_info.asset_id}_cam", cam_data)
        context.collection.objects.link(cam_obj)
        
        # 定位摄像机
        # 计算距离使平面填满画面
        plane_width = aspect  # 在Blender单位中
        fov = 2 * (cam_data.angle / 2)  # 弧度
        distance = (plane_width / 2) / (fov / 2)
        
        cam_obj.location = (0, -distance, 0)
        cam_obj.rotation_euler = (1.5708, 0, 0)  # 90度，指向平面
        
        # 设置为活动摄像机
        context.scene.camera = cam_obj
        
        return cam_obj
    
    def import_annotations(self, asset_info: AAMAssetInfo,
                          annotations: List[AAMAnnotation],
                          context: bpy.types.Context) -> List[bpy.types.Object]:
        """
        导入标注为Blender对象
        
        Args:
            asset_info: 素材信息
            annotations: 标注列表
            context: Blender上下文
            
        Returns:
            创建的对象列表
        """
        created = []
        
        # 创建标注集合
        coll_name = f"aam_{asset_info.asset_id}_annotations"
        if coll_name in bpy.data.collections:
            coll = bpy.data.collections[coll_name]
        else:
            coll = bpy.data.collections.new(coll_name)
            context.scene.collection.children.link(coll)
        
        for ann in annotations:
            obj = self._create_annotation_object(asset_info, ann, context)
            if obj:
                # 从主集合移除，添加到标注集合
                if obj.name in context.scene.collection.objects:
                    context.scene.collection.objects.unlink(obj)
                if obj.name not in coll.objects:
                    coll.objects.link(obj)
                created.append(obj)
        
        return created
    
    def _create_annotation_object(self, asset_info: AAMAssetInfo,
                                 ann: AAMAnnotation,
                                 context: bpy.types.Context) -> Optional[bpy.types.Object]:
        """创建单个标注对象"""
        
        # 将像素坐标转换为Blender坐标
        aspect = asset_info.width / asset_info.height
        bx = (ann.x - 0.5) * aspect  # X中心偏移
        by = 0.5 - ann.y  # Y翻转 (图像原点在左上，Blender在中)
        
        if ann.ann_type == 'rect':
            # 创建矩形
            bpy.ops.mesh.primitive_plane_add(size=1, location=(bx, by, 0.01))
            plane = context.active_object
            plane.name = f"aam_ann_{ann.annotation_id}"
            
            # 设置尺寸
            w = ann.width * aspect
            h = ann.height
            plane.scale = (w / 2, h / 2, 1)
            
            # 创建线框材质
            mat = bpy.data.materials.new(name=f"ann_{ann.annotation_id}_mat")
            mat.use_nodes = True
            mat.use_backface_culling = False
            mat.blend_method = 'BLEND'
            
            nodes = mat.node_tree.nodes
            links = mat.node_tree.links
            
            # 清除默认
            for node in nodes:
                nodes.remove(node)
            
            # 创建发射材质 (半透明黄色)
            output = nodes.new('ShaderNodeOutputMaterial')
            output.location = (200, 0)
            
            emission = nodes.new('ShaderNodeEmission')
            emission.location = (0, 0)
            emission.inputs['Color'].default_value = (*ann.color, 1.0)
            emission.inputs['Strength'].default_value = 1.0
            
            links.new(emission.outputs['Emission'], output.inputs['Surface'])
            
            # 设置材质属性
            mat.shadow_method = 'NONE'
            
            plane.data.materials.append(mat)
            
            # 使用线框显示
            plane.display_type = 'WIRE'
            
            return plane
            
        elif ann.ann_type == 'circle':
            # 创建圆形 (使用环)
            bpy.ops.mesh.primitive_circle_add(
                radius=0.1,
                location=(bx, by, 0.01),
                fill_type='NOTHING'
            )
            circle = context.active_object
            circle.name = f"aam_ann_{ann.annotation_id}"
            
            # 设置颜色
            mat = bpy.data.materials.new(name=f"ann_{ann.annotation_id}_mat")
            mat.use_nodes = True
            nodes = mat.node_tree.nodes
            emission = nodes.new('ShaderNodeEmission')
            emission.inputs['Color'].default_value = (*ann.color, 1.0)
            nodes.remove(nodes['Principled BSDF'])
            nodes['Material Output'].inputs['Surface'].default_value = emission.outputs['Emission']
            
            circle.data.materials.append(mat)
            circle.display_type = 'WIRE'
            
            return circle
            
        elif ann.ann_type == 'arrow':
            # 创建箭头 (简化为锥体)
            bpy.ops.mesh.primitive_cone_add(
                radius1=0.05,
                radius2=0,
                depth=0.2,
                location=(bx, by, 0.01),
            )
            arrow = context.active_object
            arrow.name = f"aam_ann_{ann.annotation_id}"
            
            return arrow
        
        return None
    
    # -------------------------------------------------------------------------
    # 导出功能
    # -------------------------------------------------------------------------
    
    def export_viewport_sequence(self, output_dir: str,
                                frame_range: Optional[Tuple[int, int]] = None,
                                resolution: Optional[Tuple[int, int]] = None,
                                context: bpy.types.Context = None) -> str:
        """
        导出视口为图像序列
        
        Args:
            output_dir: 输出目录
            frame_range: (开始帧, 结束帧)
            resolution: (宽, 高)
            context: Blender上下文
            
        Returns:
            输出路径模式
        """
        if context is None:
            context = bpy.context
        
        scene = context.scene
        
        # 保存当前设置
        orig_format = scene.render.image_settings.file_format
        orig_color = scene.render.image_settings.color_mode
        orig_path = scene.render.filepath
        orig_res_x = scene.render.resolution_x
        orig_res_y = scene.render.resolution_y
        
        # 设置输出
        scene.render.image_settings.file_format = 'PNG'
        scene.render.image_settings.color_mode = 'RGB'
        scene.render.filepath = os.path.join(output_dir, "frame_")
        
        if resolution:
            scene.render.resolution_x = resolution[0]
            scene.render.resolution_y = resolution[1]
        
        # 确定帧范围
        if frame_range:
            start, end = frame_range
        else:
            start = scene.frame_start
            end = scene.frame_end
        
        # 渲染动画
        bpy.ops.render.render(animation=True, write_file=True)
        
        # 恢复设置
        scene.render.image_settings.file_format = orig_format
        scene.render.image_settings.color_mode = orig_color
        scene.render.filepath = orig_path
        scene.render.resolution_x = orig_res_x
        scene.render.resolution_y = orig_res_y
        
        return os.path.join(output_dir, "frame_####.png")


# =============================================================================
# Blender操作符
# =============================================================================

class AAM_OT_import_asset(Operator):
    """从AAM导入素材"""
    bl_idname = "aam.import_asset"
    bl_label = "Import AAM Asset"
    bl_options = {'REGISTER', 'UNDO'}
    
    filepath: StringProperty(subtype='FILE_PATH')
    create_camera: BoolProperty(name="Create Camera", default=True)
    
    def execute(self, context):
        try:
            with open(self.filepath, 'r') as f:
                data = json.load(f)
            
            asset_info = AAMAssetInfo.from_dict(data['asset'])
            
            connector = AAMConnector()
            plane = connector.import_image_sequence(
                asset_info, context, self.create_camera
            )
            
            # 导入标注
            if 'annotations' in data:
                annotations = [AAMAnnotation.from_dict(a) for a in data['annotations']]
                connector.import_annotations(asset_info, annotations, context)
            
            self.report({'INFO'}, f"Imported: {plane.name}")
            return {'FINISHED'}
            
        except Exception as e:
            self.report({'ERROR'}, str(e))
            return {'CANCELLED'}
    
    def invoke(self, context, event):
        context.window_manager.fileselect_add(self)
        return {'RUNNING_MODAL'}


class AAM_OT_export_viewport(Operator):
    """导出视口到AAM"""
    bl_idname = "aam.export_viewport"
    bl_label = "Export to AAM"
    bl_options = {'REGISTER'}
    
    directory: StringProperty(subtype='DIR_PATH')
    
    def execute(self, context):
        try:
            connector = AAMConnector()
            output = connector.export_viewport_sequence(
                self.directory,
                context=context
            )
            
            self.report({'INFO'}, f"Exported to: {output}")
            return {'FINISHED'}
            
        except Exception as e:
            self.report({'ERROR'}, str(e))
            return {'CANCELLED'}
    
    def invoke(self, context, event):
        context.window_manager.fileselect_add(self)
        return {'RUNNING_MODAL'}


# =============================================================================
# Blender UI
# =============================================================================

class AAM_PT_panel(Panel):
    """AAM侧边栏面板"""
    bl_label = "AAM Connector"
    bl_idname = "AAM_PT_panel"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = "AAM"
    
    def draw(self, context):
        layout = self.layout
        
        # 状态显示
        box = layout.box()
        box.label(text="Status: Connected", icon='CHECKMARK')
        
        layout.separator()
        
        # 导入
        layout.label(text="Import:")
        layout.operator("aam.import_asset", icon='IMPORT')
        
        layout.separator()
        
        # 导出
        layout.label(text="Export:")
        layout.operator("aam.export_viewport", icon='EXPORT')
        
        layout.separator()
        
        # 帮助
        layout.label(text="Documentation:")
        layout.operator("wm.url_open", text="AAM Docs").url = "https://aam.readthedocs.io"


# =============================================================================
# 注册
# =============================================================================

classes = [
    AAM_OT_import_asset,
    AAM_OT_export_viewport,
    AAM_PT_panel,
]

def register():
    for cls in classes:
        bpy.utils.register_class(cls)

def unregister():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)

if __name__ == "__main__":
    register()
