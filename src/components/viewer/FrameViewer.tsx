import { useEffect, useRef, useState, useCallback } from 'react';
import type { Asset, Annotation } from '../../types';
import { useStore } from '../../hooks/useStore';
import { AnnotationCanvas } from '../annotation/AnnotationCanvas';

interface FrameViewerProps {
  asset: Asset;
  annotations: Annotation[];
  className?: string;
}

// LRU缓存实现
class FrameCache {
  private cache = new Map<number, HTMLImageElement>();
  private maxSize = 30;

  get(index: number): HTMLImageElement | undefined {
    const frame = this.cache.get(index);
    if (frame) {
      // 移动到末尾（最近使用）
      this.cache.delete(index);
      this.cache.set(index, frame);
    }
    return frame;
  }

  set(index: number, image: HTMLImageElement): void {
    if (this.cache.size >= this.maxSize) {
      // 删除最旧的
      const firstKey = this.cache.keys().next().value;
      this.cache.delete(firstKey);
    }
    this.cache.set(index, image);
  }

  clear(): void {
    this.cache.clear();
  }
}

export function FrameViewer({ asset, annotations, className = '' }: FrameViewerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const cacheRef = useRef(new FrameCache());
  
  const {
    currentFrameIndex,
    isPlaying,
    playbackSpeed,
    setCurrentFrame,
    nextFrame,
    prevFrame,
    togglePlayback,
    goToFirstFrame,
    goToLastFrame,
  } = useStore();

  const [currentImage, setCurrentImage] = useState<HTMLImageElement | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [scale, setScale] = useState(1);
  const [isDrawing, setIsDrawing] = useState(false);
  const [activeAnnotation, setActiveAnnotation] = useState<Annotation | null>(null);

  // 加载帧图像
  const loadFrame = useCallback(async (index: number) => {
    // 检查缓存
    const cached = cacheRef.current.get(index);
    if (cached && cached.complete) {
      setCurrentImage(cached);
      return;
    }

    setIsLoading(true);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const base64Data: string = await invoke('get_frame_data', {
        assetId: asset.id,
        frameIndex: index,
      });

      const img = new Image();
      img.src = base64Data;
      
      await new Promise<void>((resolve, reject) => {
        img.onload = () => resolve();
        img.onerror = reject;
      });

      cacheRef.current.set(index, img);
      setCurrentImage(img);
    } catch (error) {
      console.error('Failed to load frame:', error);
    } finally {
      setIsLoading(false);
    }
  }, [asset.id]);

  // 当前帧变化时加载
  useEffect(() => {
    loadFrame(currentFrameIndex);
  }, [currentFrameIndex, loadFrame]);

  // 播放控制
  useEffect(() => {
    if (!isPlaying) return;

    const interval = setInterval(() => {
      nextFrame();
    }, (1000 / asset.fps) / playbackSpeed);

    return () => clearInterval(interval);
  }, [isPlaying, asset.fps, playbackSpeed, nextFrame]);

  // 键盘快捷键
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // 防止在输入框中触发
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
        return;
      }

      switch (e.key) {
        case 'ArrowLeft':
          e.preventDefault();
          prevFrame();
          break;
        case 'ArrowRight':
          e.preventDefault();
          nextFrame();
          break;
        case ' ':
          e.preventDefault();
          togglePlayback();
          break;
        case 'Home':
          e.preventDefault();
          goToFirstFrame();
          break;
        case 'End':
          e.preventDefault();
          goToLastFrame();
          break;
      }

      // Ctrl+滚轮缩放
      if (e.ctrlKey && e.key === '0') {
        e.preventDefault();
        setScale(1);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [nextFrame, prevFrame, togglePlayback, goToFirstFrame, goToLastFrame]);

  // 滚轮缩放
  const handleWheel = (e: React.WheelEvent) => {
    if (e.ctrlKey) {
      e.preventDefault();
      const delta = e.deltaY > 0 ? 0.9 : 1.1;
      setScale((s) => Math.max(0.1, Math.min(s * delta, 5)));
    }
  };

  // 绘制到 canvas
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !currentImage) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // 设置 canvas 尺寸
    canvas.width = asset.resolution.width;
    canvas.height = asset.resolution.height;

    // 清空并绘制
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(currentImage, 0, 0);

    // 绘制标注
    const frameAnnotations = annotations.filter(
      (a) => a.frame_index === undefined || a.frame_index === currentFrameIndex
    );

    for (const annotation of frameAnnotations) {
      drawAnnotation(ctx, annotation);
    }
  }, [currentImage, annotations, currentFrameIndex, asset.resolution]);

  const drawAnnotation = (ctx: CanvasRenderingContext2D, annotation: Annotation) => {
    ctx.strokeStyle = annotation.color;
    ctx.fillStyle = annotation.color + '40'; // 25% opacity
    ctx.lineWidth = 2;

    const { width, height } = asset.resolution;

    switch (annotation.coordinates.type) {
      case 'rect': {
        const { x, y, width: w, height: h } = annotation.coordinates;
        const px = x * width;
        const py = y * height;
        const pw = w * width;
        const ph = h * height;
        ctx.strokeRect(px, py, pw, ph);
        ctx.fillRect(px, py, pw, ph);
        break;
      }
      case 'circle': {
        const { cx, cy, r } = annotation.coordinates;
        const px = cx * width;
        const py = cy * height;
        const pr = r * Math.max(width, height);
        ctx.beginPath();
        ctx.arc(px, py, pr, 0, Math.PI * 2);
        ctx.stroke();
        ctx.fill();
        break;
      }
      case 'arrow': {
        const { x1, y1, x2, y2 } = annotation.coordinates;
        const px1 = x1 * width;
        const py1 = y1 * height;
        const px2 = x2 * width;
        const py2 = y2 * height;
        drawArrow(ctx, px1, py1, px2, py2);
        break;
      }
      case 'text': {
        const { x, y } = annotation.coordinates;
        const px = x * width;
        const py = y * height;
        if (annotation.label) {
          ctx.font = '14px sans-serif';
          ctx.fillStyle = annotation.color;
          ctx.fillText(annotation.label, px, py);
        }
        break;
      }
      case 'stroke': {
        const { points } = annotation.coordinates;
        if (points.length < 2) break;
        ctx.beginPath();
        ctx.moveTo(points[0][0] * width, points[0][1] * height);
        for (let i = 1; i < points.length; i++) {
          ctx.lineTo(points[i][0] * width, points[i][1] * height);
        }
        ctx.stroke();
        break;
      }
    }
  };

  const drawArrow = (ctx: CanvasRenderingContext2D, x1: number, y1: number, x2: number, y2: number) => {
    const headLength = 10;
    const angle = Math.atan2(y2 - y1, x2 - x1);

    ctx.beginPath();
    ctx.moveTo(x1, y1);
    ctx.lineTo(x2, y2);
    ctx.stroke();

    // 箭头头部
    ctx.beginPath();
    ctx.moveTo(x2, y2);
    ctx.lineTo(
      x2 - headLength * Math.cos(angle - Math.PI / 6),
      y2 - headLength * Math.sin(angle - Math.PI / 6)
    );
    ctx.moveTo(x2, y2);
    ctx.lineTo(
      x2 - headLength * Math.cos(angle + Math.PI / 6),
      y2 - headLength * Math.sin(angle + Math.PI / 6)
    );
    ctx.stroke();
  };

  // 时间轴点击
  const handleTimelineClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const percentage = Math.max(0, Math.min(x / rect.width, 1));
    const targetFrame = Math.floor(percentage * (asset.frame_count - 1));
    setCurrentFrame(targetFrame);
  };

  const currentAnnotationsForFrame = annotations.filter(
    (a) => a.frame_index === undefined || a.frame_index === currentFrameIndex
  );

  return (
    <div className={`flex flex-col ${className}`}>
      {/* 主视图区 */}
      <div
        ref={containerRef}
        className="flex-1 bg-neutral-950 relative overflow-hidden flex items-center justify-center"
        onWheel={handleWheel}
      >
        {/* 加载指示器 */}
        {isLoading && (
          <div className="absolute inset-0 flex items-center justify-center bg-neutral-950/50 z-10">
            <div className="w-8 h-8 border-2 border-neutral-700 border-t-white rounded-full animate-spin" />
          </div>
        )}

        {/* 画布容器 */}
        <div
          style={{
            transform: `scale(${scale})`,
            transition: 'transform 0.1s ease-out',
          }}
          className="relative"
        >
          <canvas
            ref={canvasRef}
            className="max-w-full max-h-full"
            style={{
              imageRendering: 'pixelated',
            }}
          />
          
          {/* 标注绘制层 */}
          <AnnotationCanvas
            asset={asset}
            annotations={currentAnnotationsForFrame}
            currentFrameIndex={currentFrameIndex}
            isDrawing={isDrawing}
            onAnnotationCreate={(coords) => {
              useStore.getState().addAnnotation({
                asset_id: asset.id,
                frame_index: currentFrameIndex,
                annotation_type: coords.type === 'rect' ? 'rect' : 'stroke',
                coordinates: coords,
                color: '#ff0000',
              });
            }}
            className="absolute inset-0"
          />
        </div>

        {/* 缩放指示器 */}
        <div className="absolute bottom-4 left-4 bg-neutral-900/80 px-2 py-1 rounded text-xs text-neutral-400">
          {Math.round(scale * 100)}%
        </div>

        {/* 标注数量 */}
        {currentAnnotationsForFrame.length > 0 && (
          <div className="absolute top-4 right-4 bg-neutral-900/80 px-3 py-1 rounded-full text-xs">
            <span className="text-neutral-400">Annotations:</span>
            <span className="ml-1 text-white">{currentAnnotationsForFrame.length}</span>
          </div>
        )}
      </div>

      {/* 控制栏 */}
      <div className="h-16 bg-neutral-900 border-t border-neutral-800 flex items-center px-4 gap-4">
        {/* 播放控制 */}
        <button
          onClick={togglePlayback}
          className="w-10 h-10 flex items-center justify-center bg-white text-neutral-900 rounded-lg hover:bg-neutral-200 transition-colors"
        >
          {isPlaying ? (
            <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
              <rect x="6" y="4" width="4" height="16" />
              <rect x="14" y="4" width="4" height="16" />
            </svg>
          ) : (
            <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
              <path d="M8 5v14l11-7z" />
            </svg>
          )}
        </button>

        {/* 帧导航 */}
        <div className="flex items-center gap-1">
          <button
            onClick={goToFirstFrame}
            className="px-2 py-1 text-neutral-400 hover:text-white transition-colors"
            title="First frame (Home)"
          >
            |←
          </button>
          <button
            onClick={prevFrame}
            className="px-2 py-1 text-neutral-400 hover:text-white transition-colors"
            title="Previous (←)"
          >
            ←
          </button>
          <button
            onClick={nextFrame}
            className="px-2 py-1 text-neutral-400 hover:text-white transition-colors"
            title="Next (→)"
          >
            →
          </button>
          <button
            onClick={goToLastFrame}
            className="px-2 py-1 text-neutral-400 hover:text-white transition-colors"
            title="Last frame (End)"
          >
            →|
          </button>
        </div>

        {/* 帧信息 */}
        <div className="text-sm">
          <span className="text-white font-medium">{currentFrameIndex + 1}</span>
          <span className="text-neutral-500"> / {asset.frame_count}</span>
        </div>

        {/* 时间轴 */}
        <div
          className="flex-1 h-6 bg-neutral-800 rounded cursor-pointer relative"
          onClick={handleTimelineClick}
        >
          {/* 进度条 */}
          <div
            className="absolute top-0 left-0 h-full bg-white/20 rounded"
            style={{
              width: `${((currentFrameIndex + 1) / asset.frame_count) * 100}%`,
            }}
          />
          {/* 播放头 */}
          <div
            className="absolute top-0 w-1 h-full bg-white rounded"
            style={{
              left: `${((currentFrameIndex + 1) / asset.frame_count) * 100}%`,
            }}
          />
        </div>

        {/* 速度控制 */}
        <select
          value={playbackSpeed}
          onChange={(e) => useStore.getState().setPlaybackSpeed(parseFloat(e.target.value))}
          className="bg-neutral-800 text-white text-sm px-2 py-1 rounded border border-neutral-700"
        >
          <option value={0.25}>0.25x</option>
          <option value={0.5}>0.5x</option>
          <option value={1}>1x</option>
          <option value={2}>2x</option>
          <option value={4}>4x</option>
        </select>

        {/* 标注工具切换 */}
        <button
          onClick={() => setIsDrawing(!isDrawing)}
          className={`px-3 py-1 rounded text-sm transition-colors ${
            isDrawing
              ? 'bg-blue-500 text-white'
              : 'bg-neutral-800 text-neutral-400 hover:text-white'
          }`}
        >
          Draw
        </button>
      </div>
    </div>
  );
}
