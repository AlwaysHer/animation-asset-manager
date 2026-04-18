// Tauri API 封装

import { invoke } from '@tauri-apps/api/core';
import type { Asset, Annotation, ExportFormat, FrameRange } from '../types';

// 素材管理
export async function createAsset(sourcePath: string): Promise<Asset> {
  return invoke('create_asset', { sourcePath });
}

export async function getAsset(id: string): Promise<Asset | null> {
  return invoke('get_asset', { id });
}

export async function getAllAssets(): Promise<Asset[]> {
  return invoke('get_all_assets');
}

export async function deleteAsset(id: string): Promise<boolean> {
  return invoke('delete_asset', { id });
}

// 帧管理
export async function getFramePath(assetId: string, frameIndex: number): Promise<string> {
  return invoke('get_frame_path', { assetId, frameIndex });
}

export async function getFrameData(assetId: string, frameIndex: number): Promise<string> {
  return invoke('get_frame_data', { assetId, frameIndex });
}

// 标注管理
export async function createAnnotation(annotation: Annotation): Promise<void> {
  return invoke('create_annotation', { annotation });
}

export async function getAnnotations(assetId: string): Promise<Annotation[]> {
  return invoke('get_annotations', { assetId });
}

export async function getAnnotationsForFrame(assetId: string, frameIndex: number): Promise<Annotation[]> {
  return invoke('get_annotations_for_frame', { assetId, frameIndex });
}

export async function deleteAnnotation(id: string): Promise<boolean> {
  return invoke('delete_annotation', { id });
}

// 导出
export async function exportAsset(
  assetId: string,
  format: ExportFormat,
  outputPath: string,
  frameRange: FrameRange,
  includeAnnotations: boolean
): Promise<string> {
  return invoke('export_asset', {
    assetId,
    format,
    outputPath,
    frameRange,
    includeAnnotations,
  });
}

export async function generateImportScript(
  dcc: 'maya' | 'blender',
  sequencePath: string,
  fps: number,
  frameCount: number
): Promise<string> {
  return invoke('generate_import_script', {
    dcc,
    sequencePath,
    fps,
    frameCount,
  });
}
