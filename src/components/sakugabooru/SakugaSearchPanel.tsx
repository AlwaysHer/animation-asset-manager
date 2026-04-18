// Sakugabooru 搜索面板

import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Asset } from '../../types';

interface SakugaPost {
  id: number;
  tags: string;
  file_url: string;
  preview_url: string;
  sample_url?: string;
  width: number;
  height: number;
  source?: string;
  rating: string;
  score: number;
  created_at: string;
}

interface SakugaSearchPanelProps {
  onImport: (asset: Asset) => void;
}

const PRESET_TAGS = [
  { label: '攻击', tag: 'attack' },
  { label: '特效', tag: 'effects' },
  { label: '跑步', tag: 'running' },
  { label: '烟雾', tag: 'smoke' },
  { label: '火焰', tag: 'fire' },
  { label: '液体', tag: 'liquid' },
  { label: '原画', tag: 'genga' },
  { label: '动画', tag: 'animated' },
];

export function SakugaSearchPanel({ onImport }: SakugaSearchPanelProps) {
  const [query, setQuery] = useState('');
  const [posts, setPosts] = useState<SakugaPost[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [downloadingId, setDownloadingId] = useState<number | null>(null);
  const [page, setPage] = useState(1);
  const [hasMore, setHasMore] = useState(true);

  const search = useCallback(async (newPage = 1) => {
    if (!query.trim()) return;
    
    setIsLoading(true);
    try {
      const results: SakugaPost[] = await invoke('search_sakugabooru', {
        query: query.trim(),
        page: newPage,
        limit: 20,
      });
      
      if (newPage === 1) {
        setPosts(results);
      } else {
        setPosts(prev => [...prev, ...results]);
      }
      
      setHasMore(results.length === 20);
      setPage(newPage);
    } catch (error) {
      console.error('Search failed:', error);
      alert(`搜索失败: ${error}`);
    } finally {
      setIsLoading(false);
    }
  }, [query]);

  const downloadPost = async (post: SakugaPost) => {
    setDownloadingId(post.id);
    try {
      const asset: Asset = await invoke('download_sakuga_post', { post });
      onImport(asset);
      alert('下载完成并已导入素材库！');
    } catch (error) {
      console.error('Download failed:', error);
      alert(`下载失败: ${error}`);
    } finally {
      setDownloadingId(null);
    }
  };

  const loadMore = () => {
    if (!isLoading && hasMore) {
      search(page + 1);
    }
  };

  const applyPresetTag = (tag: string) => {
    setQuery(prev => {
      const tags = prev.trim().split(/\s+/).filter(Boolean);
      if (tags.includes(tag)) return prev;
      return [...tags, tag].join(' ');
    });
  };

  const formatFileSize = (url: string) => {
    // Sakugabooru文件通常较大，估算一下
    if (url.endsWith('.webm')) return 'WebM';
    if (url.endsWith('.gif')) return 'GIF';
    if (url.endsWith('.mp4')) return 'MP4';
    return 'Video';
  };

  return (
    <div className="h-full flex flex-col bg-neutral-900">
      {/* 搜索头部 */}
      <div className="p-4 border-b border-neutral-800">
        <h2 className="text-lg font-semibold mb-3">Sakugabooru 搜索</h2>
        
        {/* 搜索框 */}
        <div className="flex gap-2 mb-3">
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && search(1)}
            placeholder="输入标签搜索，如: attack effects..."
            className="flex-1 bg-neutral-800 text-white px-3 py-2 rounded border border-neutral-700 focus:border-white focus:outline-none"
          />
          <button
            onClick={() => search(1)}
            disabled={isLoading || !query.trim()}
            className="px-4 py-2 bg-white text-neutral-900 rounded font-medium hover:bg-neutral-200 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {isLoading ? '...' : '搜索'}
          </button>
        </div>

        {/* 预设标签 */}
        <div className="flex flex-wrap gap-1">
          {PRESET_TAGS.map(({ label, tag }) => (
            <button
              key={tag}
              onClick={() => applyPresetTag(tag)}
              className="px-2 py-1 text-xs bg-neutral-800 hover:bg-neutral-700 rounded text-neutral-300 transition-colors"
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      {/* 结果列表 */}
      <div className="flex-1 overflow-y-auto p-4">
        {posts.length === 0 && !isLoading && (
          <div className="text-center text-neutral-500 py-8">
            <p>搜索Sakugabooru获取高质量作画</p>
            <p className="text-sm mt-2">支持标签组合搜索</p>
          </div>
        )}

        <div className="grid grid-cols-2 gap-3">
          {posts.map((post) => (
            <div
              key={post.id}
              className="bg-neutral-800 rounded-lg overflow-hidden group"
            >
              {/* 预览图 */}
              <div className="aspect-video bg-neutral-950 relative">
                <img
                  src={post.preview_url}
                  alt={`Post ${post.id}`}
                  loading="lazy"
                  className="w-full h-full object-cover"
                />
                
                {/* 悬停信息 */}
                <div className="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity flex flex-col justify-end p-2">
                  <div className="text-xs text-white">
                    <p>{post.width} × {post.height}</p>
                    <p>{formatFileSize(post.file_url)} · Score: {post.score}</p>
                  </div>
                </div>

                {/* 下载按钮 */}
                <button
                  onClick={() => downloadPost(post)}
                  disabled={downloadingId === post.id}
                  className="absolute top-2 right-2 px-3 py-1 bg-white text-neutral-900 text-xs rounded font-medium opacity-0 group-hover:opacity-100 transition-opacity hover:bg-neutral-200 disabled:opacity-50"
                >
                  {downloadingId === post.id ? '下载中...' : '导入'}
                </button>
              </div>

              {/* 标签预览 */}
              <div className="p-2">
                <div className="flex flex-wrap gap-1">
                  {post.tags.split(' ').slice(0, 6).map((tag) => (
                    <span
                      key={tag}
                      onClick={() => {
                        setQuery(tag);
                        search(1);
                      }}
                      className="text-xs text-neutral-400 hover:text-white cursor-pointer"
                    >
                      {tag}
                    </span>
                  ))}
                  {post.tags.split(' ').length > 6 && (
                    <span className="text-xs text-neutral-500">
                      +{post.tags.split(' ').length - 6}
                    </span>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>

        {/* 加载更多 */}
        {posts.length > 0 && (
          <div className="mt-4 text-center">
            {hasMore ? (
              <button
                onClick={loadMore}
                disabled={isLoading}
                className="px-4 py-2 bg-neutral-800 hover:bg-neutral-700 rounded text-sm text-neutral-300 transition-colors"
              >
                {isLoading ? '加载中...' : '加载更多'}
              </button>
            ) : (
              <p className="text-xs text-neutral-500">没有更多结果</p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
