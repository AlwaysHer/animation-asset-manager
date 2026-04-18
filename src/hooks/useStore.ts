// Zustand 状态管理

import { create } from 'zustand';
import type { Asset, Annotation, ExportConfig, ExportFormat } from '../types';
import * as api from '../services/api';

interface AppState {
  // 状态
  assets: Asset[];
  selectedAsset: Asset | null;
  isLoadingAssets: boolean;
  currentAnnotations: Annotation[];
  currentFrameIndex: number;
  isPlaying: boolean;
  playbackSpeed: number;
  
  // 操作
  loadAssets: () => Promise<void>;
  selectAsset: (asset: Asset | null) => Promise<void>;
  importAsset: (path: string) => Promise<void>;
  deleteAsset: (id: string) => Promise<void>;
  
  // 帧控制
  setCurrentFrame: (index: number) => void;
  nextFrame: () => void;
  prevFrame: () => void;
  togglePlayback: () => void;
  setPlaybackSpeed: (speed: number) => void;
  goToFirstFrame: () => void;
  goToLastFrame: () => void;
  
  // 标注
  addAnnotation: (annotation: Omit<Annotation, 'id' | 'created_at' | 'created_by'>) => Promise<void>;
  removeAnnotation: (id: string) => Promise<void>;
  refreshAnnotations: () => Promise<void>;
  
  // 导出
  exportAsset: (config: ExportConfig) => Promise<string>;
  generateImportScript: (dcc: 'maya' | 'blender') => Promise<string>;
}

export const useStore = create<AppState>((set, get) => ({
  // 初始状态
  assets: [],
  selectedAsset: null,
  isLoadingAssets: false,
  currentAnnotations: [],
  currentFrameIndex: 0,
  isPlaying: false,
  playbackSpeed: 1,

  // 加载所有素材
  loadAssets: async () => {
    set({ isLoadingAssets: true });
    try {
      const assets = await api.getAllAssets();
      set({ assets });
    } catch (error) {
      console.error('Failed to load assets:', error);
    } finally {
      set({ isLoadingAssets: false });
    }
  },

  // 选择素材
  selectAsset: async (asset) => {
    set({ 
      selectedAsset: asset, 
      currentFrameIndex: 0,
      currentAnnotations: [] 
    });
    
    if (asset) {
      await get().refreshAnnotations();
    }
  },

  // 导入素材
  importAsset: async (path) => {
    try {
      const asset = await api.createAsset(path);
      set((state) => ({ 
        assets: [asset, ...state.assets],
        selectedAsset: asset 
      }));
      await get().refreshAnnotations();
    } catch (error) {
      console.error('Failed to import asset:', error);
      throw error;
    }
  },

  // 删除素材
  deleteAsset: async (id) => {
    try {
      await api.deleteAsset(id);
      set((state) => ({
        assets: state.assets.filter((a) => a.id !== id),
        selectedAsset: state.selectedAsset?.id === id ? null : state.selectedAsset,
      }));
    } catch (error) {
      console.error('Failed to delete asset:', error);
    }
  },

  // 帧控制
  setCurrentFrame: (index) => {
    const { selectedAsset } = get();
    if (!selectedAsset) return;
    
    const clampedIndex = Math.max(0, Math.min(index, selectedAsset.frame_count - 1));
    set({ currentFrameIndex: clampedIndex });
  },

  nextFrame: () => {
    const { currentFrameIndex, selectedAsset } = get();
    if (!selectedAsset) return;
    
    const nextIndex = (currentFrameIndex + 1) % selectedAsset.frame_count;
    set({ currentFrameIndex: nextIndex });
  },

  prevFrame: () => {
    const { currentFrameIndex, selectedAsset } = get();
    if (!selectedAsset) return;
    
    const prevIndex = currentFrameIndex === 0 
      ? selectedAsset.frame_count - 1 
      : currentFrameIndex - 1;
    set({ currentFrameIndex: prevIndex });
  },

  togglePlayback: () => {
    set((state) => ({ isPlaying: !state.isPlaying }));
  },

  setPlaybackSpeed: (speed) => {
    set({ playbackSpeed: Math.max(0.25, Math.min(speed, 4)) });
  },

  goToFirstFrame: () => {
    set({ currentFrameIndex: 0 });
  },

  goToLastFrame: () => {
    const { selectedAsset } = get();
    if (selectedAsset) {
      set({ currentFrameIndex: selectedAsset.frame_count - 1 });
    }
  },

  // 标注
  addAnnotation: async (annotationData) => {
    const { selectedAsset } = get();
    if (!selectedAsset) return;

    const annotation: Annotation = {
      ...annotationData,
      id: crypto.randomUUID(),
      created_at: new Date().toISOString(),
      created_by: 'user',
    };

    try {
      await api.createAnnotation(annotation);
      await get().refreshAnnotations();
    } catch (error) {
      console.error('Failed to add annotation:', error);
      throw error;
    }
  },

  removeAnnotation: async (id) => {
    try {
      await api.deleteAnnotation(id);
      set((state) => ({
        currentAnnotations: state.currentAnnotations.filter((a) => a.id !== id),
      }));
    } catch (error) {
      console.error('Failed to remove annotation:', error);
    }
  },

  refreshAnnotations: async () => {
    const { selectedAsset } = get();
    if (!selectedAsset) return;

    try {
      const annotations = await api.getAnnotations(selectedAsset.id);
      set({ currentAnnotations: annotations });
    } catch (error) {
      console.error('Failed to load annotations:', error);
    }
  },

  // 导出
  exportAsset: async (config) => {
    const { selectedAsset } = get();
    if (!selectedAsset) throw new Error('No asset selected');

    return api.exportAsset(
      selectedAsset.id,
      config.format,
      config.output_path,
      config.frame_range,
      config.include_annotations
    );
  },

  generateImportScript: async (dcc) => {
    const { selectedAsset } = get();
    if (!selectedAsset) throw new Error('No asset selected');

    return api.generateImportScript(
      dcc,
      selectedAsset.frame_directory,
      selectedAsset.fps,
      selectedAsset.frame_count
    );
  },
}));
