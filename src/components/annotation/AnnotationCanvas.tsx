import { useRef, useEffect, useState, useCallback } from 'react';
import type { Asset, AnnotationCoords } from '../../types';

interface AnnotationCanvasProps {
  asset: Asset;
  annotations: any[];
  currentFrameIndex: number;
  isDrawing: boolean;
  onAnnotationCreate?: (coords: AnnotationCoords) => void;
  className?: string;
}

type DrawingMode = 'rect' | 'circle' | 'arrow' | 'text' | 'stroke';

interface Point {
  x: number;
  y: number;
}

export function AnnotationCanvas({
  asset: _asset,
  annotations,
  currentFrameIndex: _currentFrameIndex,
  isDrawing,
  onAnnotationCreate,
  className = '',
}: AnnotationCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [mode, setMode] = useState<DrawingMode>('rect');
  const [isDragging, setIsDragging] = useState(false);
  const [startPoint, setStartPoint] = useState<Point | null>(null);
  const [currentPoint, setCurrentPoint] = useState<Point | null>(null);
  const [strokePoints, setStrokePoints] = useState<Point[]>([]);
  const [textInput, setTextInput] = useState('');
  const [textPosition, setTextPosition] = useState<Point | null>(null);

  // 坐标转换：屏幕像素 -> 归一化 (0-1)
  const normalizePoint = useCallback(
    (x: number, y: number): Point => {
      const canvas = canvasRef.current;
      if (!canvas) return { x: 0, y: 0 };
      const rect = canvas.getBoundingClientRect();
      return {
        x: (x - rect.left) / rect.width,
        y: (y - rect.top) / rect.height,
      };
    },
    []
  );

  // 坐标转换：归一化 -> 屏幕像素
  const denormalizePoint = useCallback(
    (point: Point): Point => {
      const canvas = canvasRef.current;
      if (!canvas) return { x: 0, y: 0 };
      const rect = canvas.getBoundingClientRect();
      return {
        x: point.x * rect.width,
        y: point.y * rect.height,
      };
    },
    []
  );

  // 设置 canvas 尺寸匹配父元素
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const resize = () => {
      const parent = canvas.parentElement;
      if (parent) {
        canvas.width = parent.clientWidth;
        canvas.height = parent.clientHeight;
      }
    };

    resize();
    window.addEventListener('resize', resize);
    return () => window.removeEventListener('resize', resize);
  }, []);

  // 绘制标注预览
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // 清空
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // 绘制当前绘制中的形状
    if (isDragging && startPoint && currentPoint) {
      ctx.strokeStyle = '#ff0000';
      ctx.lineWidth = 2;
      ctx.setLineDash([5, 5]);

      const start = denormalizePoint(startPoint);
      const current = denormalizePoint(currentPoint);

      switch (mode) {
        case 'rect': {
          const width = current.x - start.x;
          const height = current.y - start.y;
          ctx.strokeRect(start.x, start.y, width, height);
          break;
        }
        case 'circle': {
          const radius = Math.sqrt(
            Math.pow(current.x - start.x, 2) + Math.pow(current.y - start.y, 2)
          );
          ctx.beginPath();
          ctx.arc(start.x, start.y, radius, 0, Math.PI * 2);
          ctx.stroke();
          break;
        }
        case 'arrow': {
          drawArrow(ctx, start.x, start.y, current.x, current.y);
          break;
        }
      }

      ctx.setLineDash([]);
    }

    // 绘制笔迹
    if (strokePoints.length > 0) {
      ctx.strokeStyle = '#ff0000';
      ctx.lineWidth = 2;
      ctx.lineCap = 'round';
      ctx.lineJoin = 'round';

      ctx.beginPath();
      const first = denormalizePoint(strokePoints[0]);
      ctx.moveTo(first.x, first.y);

      for (let i = 1; i < strokePoints.length; i++) {
        const point = denormalizePoint(strokePoints[i]);
        ctx.lineTo(point.x, point.y);
      }

      ctx.stroke();
    }

    // 绘制已有标注的命中框（用于选择）
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.3)';
    ctx.lineWidth = 1;

    for (const annotation of annotations) {
      if (!annotation.coordinates) continue;

      const coords = annotation.coordinates;
      if (coords.type === 'rect') {
        const x = coords.x * canvas.width;
        const y = coords.y * canvas.height;
        const w = coords.width * canvas.width;
        const h = coords.height * canvas.height;
        ctx.strokeRect(x, y, w, h);
      }
    }
  }, [isDragging, startPoint, currentPoint, strokePoints, annotations, mode, denormalizePoint]);

  const drawArrow = (
    ctx: CanvasRenderingContext2D,
    x1: number,
    y1: number,
    x2: number,
    y2: number
  ) => {
    const headLength = 10;
    const angle = Math.atan2(y2 - y1, x2 - x1);

    ctx.beginPath();
    ctx.moveTo(x1, y1);
    ctx.lineTo(x2, y2);
    ctx.stroke();

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

  // 鼠标事件处理
  const handleMouseDown = (e: React.MouseEvent) => {
    if (!isDrawing) return;

    const point = normalizePoint(e.clientX, e.clientY);
    setIsDragging(true);
    setStartPoint(point);
    setCurrentPoint(point);

    if (mode === 'stroke') {
      setStrokePoints([point]);
    } else if (mode === 'text') {
      setTextPosition(point);
    }
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!isDragging || !isDrawing) return;

    const point = normalizePoint(e.clientX, e.clientY);
    setCurrentPoint(point);

    if (mode === 'stroke') {
      setStrokePoints((prev) => [...prev, point]);
    }
  };

  const handleMouseUp = () => {
    if (!isDragging || !isDrawing) return;

    setIsDragging(false);

    if (startPoint && currentPoint) {
      switch (mode) {
        case 'rect': {
          const width = Math.abs(currentPoint.x - startPoint.x);
          const height = Math.abs(currentPoint.y - startPoint.y);
          const x = Math.min(startPoint.x, currentPoint.x);
          const y = Math.min(startPoint.y, currentPoint.y);

          if (width > 0.01 && height > 0.01) {
            onAnnotationCreate?.({
              type: 'rect',
              x,
              y,
              width,
              height,
            });
          }
          break;
        }

        case 'circle': {
          const radius = Math.sqrt(
            Math.pow(currentPoint.x - startPoint.x, 2) +
              Math.pow(currentPoint.y - startPoint.y, 2)
          );

          if (radius > 0.01) {
            onAnnotationCreate?.({
              type: 'circle',
              cx: startPoint.x,
              cy: startPoint.y,
              r: radius,
            });
          }
          break;
        }

        case 'arrow': {
          const distance = Math.sqrt(
            Math.pow(currentPoint.x - startPoint.x, 2) +
              Math.pow(currentPoint.y - startPoint.y, 2)
          );

          if (distance > 0.01) {
            onAnnotationCreate?.({
              type: 'arrow',
              x1: startPoint.x,
              y1: startPoint.y,
              x2: currentPoint.x,
              y2: currentPoint.y,
            });
          }
          break;
        }

        case 'stroke': {
          if (strokePoints.length > 2) {
            onAnnotationCreate?.({
              type: 'stroke',
              points: strokePoints.map((p) => [p.x, p.y]),
            });
          }
          setStrokePoints([]);
          break;
        }
      }
    }

    setStartPoint(null);
    setCurrentPoint(null);
  };

  const handleTextSubmit = () => {
    if (textPosition && textInput.trim()) {
      onAnnotationCreate?.({
        type: 'text',
        x: textPosition.x,
        y: textPosition.y,
      });
      setTextInput('');
      setTextPosition(null);
    }
  };

  if (!isDrawing) {
    return (
      <canvas
        ref={canvasRef}
        className={`pointer-events-none ${className}`}
      />
    );
  }

  return (
    <div className={`relative ${className}`}>
      {/* 工具栏 */}
      <div className="absolute top-4 left-1/2 -translate-x-1/2 flex gap-1 bg-neutral-900/90 p-1 rounded-lg z-10">
        {(['rect', 'circle', 'arrow', 'stroke', 'text'] as DrawingMode[]).map((m) => (
          <button
            key={m}
            onClick={() => setMode(m)}
            className={`px-3 py-1 rounded text-sm transition-colors ${
              mode === m
                ? 'bg-white text-neutral-900'
                : 'text-neutral-400 hover:text-white'
            }`}
          >
            {m === 'rect' && '□'}
            {m === 'circle' && '○'}
            {m === 'arrow' && '→'}
            {m === 'stroke' && '✎'}
            {m === 'text' && 'T'}
          </button>
        ))}
      </div>

      {/* 绘制画布 */}
      <canvas
        ref={canvasRef}
        className="cursor-crosshair"
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
      />

      {/* 文本输入框 */}
      {textPosition && (
        <div
          className="absolute z-20"
          style={{
            left: textPosition.x * 100 + '%',
            top: textPosition.y * 100 + '%',
          }}
        >
          <input
            type="text"
            autoFocus
            placeholder="Enter text..."
            value={textInput}
            onChange={(e) => setTextInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleTextSubmit();
              if (e.key === 'Escape') {
                setTextInput('');
                setTextPosition(null);
              }
            }}
            onBlur={handleTextSubmit}
            className="bg-neutral-900 text-white px-2 py-1 rounded border border-neutral-700 text-sm min-w-[100px]"
          />
        </div>
      )}

      {/* 提示 */}
      <div className="absolute bottom-4 left-1/2 -translate-x-1/2 bg-neutral-900/90 px-3 py-1 rounded text-xs text-neutral-400">
        {mode === 'rect' && 'Drag to draw rectangle'}
        {mode === 'circle' && 'Drag to draw circle'}
        {mode === 'arrow' && 'Drag to draw arrow'}
        {mode === 'stroke' && 'Drag to draw freehand'}
        {mode === 'text' && 'Click to place text'}
      </div>
    </div>
  );
}
