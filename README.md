# Animation Asset Manager (AAM)

跨平台动画素材管理工具 - 支持GIF逐帧预览、标注、H264导出。

## 功能

- **素材导入**: GIF、WebM、MP4、MOV、PNG、JPG
- **逐帧预览**: 支持播放/暂停、键盘导航、缩放
- **标注系统**: 矩形、圆形、箭头、手绘笔迹
- **导出选项**: PNG序列、H264、ProRes、GIF、WebM
- **DCC友好**: 生成Maya/Blender导入辅助脚本

## 技术栈

- **Framework**: Tauri v2 (Rust + React + TypeScript)
- **UI**: Tailwind CSS + Framer Motion
- **State**: Zustand
- **Storage**: SQLite + 文件系统

## 项目结构

```
animation-asset-manager/
├── src/                        # React前端
│   ├── components/
│   │   ├── viewer/
│   │   │   └── FrameViewer.tsx      # 逐帧预览器
│   │   └── annotation/
│   │       └── AnnotationCanvas.tsx # 标注画布
│   ├── services/
│   │   └── api.ts                   # Tauri命令封装
│   ├── hooks/
│   │   └── useStore.ts              # Zustand状态管理
│   ├── types/
│   │   └── index.ts                 # TypeScript类型
│   ├── App.tsx                      # 主界面
│   ├── main.tsx                     # 入口
│   └── index.css                    # Tailwind样式
├── src-tauri/src/             # Rust后端
│   ├── main.rs                # 入口点
│   ├── models/mod.rs          # 数据模型
│   ├── commands/mod.rs        # Tauri命令
│   ├── services/              # 服务层
│   │   ├── storage_service.rs # SQLite存储
│   │   ├── decoder_service.rs # 解码服务
│   │   └── encoder_service.rs # 编码服务
│   └── dcc/mod.rs             # 导出配置
├── package.json
├── tsconfig.json
├── vite.config.ts
└── tailwind.config.js
```

## 前置要求

- Rust + Cargo
- Node.js + npm
- FFmpeg (系统安装)

## 构建

```bash
# 安装前端依赖
npm install

# 开发模式
npm run tauri dev

# 生产构建
npm run tauri build
```

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| ← | 上一帧 |
| → | 下一帧 |
| Space | 播放/暂停 |
| Home | 跳转到第一帧 |
| End | 跳转到最后一帧 |
| Ctrl+滚轮 | 缩放 |

## 导出格式

- **PNG Sequence**: 无损图像序列，DCC软件通用
- **H264**: 压缩视频，适合预览
- **ProRes**: 专业后期格式
- **GIF**: 网页展示
- **WebM**: 现代浏览器支持

## Maya/Blender集成

应用提供导入辅助脚本生成功能：
1. 在右侧属性面板点击 "Copy Maya Script" 或 "Copy Blender Script"
2. 在对应软件的Python控制台中粘贴运行
3. 脚本会自动创建imagePlane并设置图像序列

## 架构设计

### Deep Modules模式
- **StorageService**: `get_asset(id) → Asset` - 隐藏SQLite复杂性
- **DecoderService**: `decode(file) → Frame[]` - 隐藏FFmpeg调用
- **EncoderService**: `encode(asset, format) → Path` - 隐藏编码参数

### Define Errors Out策略
- 帧索引越界 → 自动裁剪到有效范围
- 编码失败 → 降级为PNG序列输出

## License

MIT# animation-asset-manager
