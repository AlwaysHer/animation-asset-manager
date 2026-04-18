import { useEffect, useState } from 'react';
import { useStore } from './hooks/useStore';
import { FrameViewer } from './components/viewer/FrameViewer';
import { SakugaSearchPanel } from './components/sakugabooru';
import type { Asset, ExportFormat } from './types';

type TabType = 'local' | 'sakugabooru';

function App() {
  const {
    assets,
    selectedAsset,
    isLoadingAssets,
    currentAnnotations,
    loadAssets,
    selectAsset,
    importAsset,
    exportAsset,
    generateImportScript,
  } = useStore();

  const [activeTab, setActiveTab] = useState<TabType>('local');

  // 初始化加载
  useEffect(() => {
    loadAssets();
  }, []);

  // 导入文件处理
  const handleImport = async () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.gif,.webm,.mp4,.mov,.png,.jpg,.jpeg';

    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (file) {
        const path = (file as any).path || file.name;
        await importAsset(path);
      }
    };

    input.click();
  };

  // 处理导出
  const handleExport = async (format: ExportFormat) => {
    if (!selectedAsset) return;

    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const ext = format === 'png_sequence' ? '' : `.${format.replace('_', '.')}`;
    const outputPath = `${selectedAsset.id}_${timestamp}${ext}`;

    const config = {
      format,
      frame_range: { type: 'all' as const },
      include_annotations: false,
      output_path: outputPath,
    };

    try {
      const result = await exportAsset(config);
      alert(`导出成功: ${result}`);
    } catch (error) {
      alert(`导出失败: ${error}`);
    }
  };

  // 生成导入脚本
  const handleGenerateScript = async (dcc: 'maya' | 'blender') => {
    try {
      const script = await generateImportScript(dcc);
      await navigator.clipboard.writeText(script);
      alert(`${dcc} 导入脚本已复制到剪贴板！在${dcc}的Python控制台中粘贴运行。`);
    } catch (error) {
      alert(`生成脚本失败: ${error}`);
    }
  };

  // 处理Sakugabooru导入
  const handleSakugaImport = (asset: Asset) => {
    loadAssets(); // 刷新列表
    selectAsset(asset);
    setActiveTab('local'); // 切换到本地视图
  };

  return (
    <div className="h-screen w-screen bg-neutral-950 text-white flex overflow-hidden">
      {/* 左侧栏 */}
      <aside className="w-80 bg-neutral-900 border-r border-neutral-800 flex flex-col">
        {/* 标签切换 */}
        <div className="flex border-b border-neutral-800">
          <button
            onClick={() => setActiveTab('local')}
            className={`flex-1 py-3 text-sm font-medium transition-colors ${
              activeTab === 'local'
                ? 'bg-white/10 text-white'
                : 'text-neutral-400 hover:text-white'
            }`}
          >
            本地素材
          </button>
          <button
            onClick={() => setActiveTab('sakugabooru')}
            className={`flex-1 py-3 text-sm font-medium transition-colors ${
              activeTab === 'sakugabooru'
                ? 'bg-white/10 text-white'
                : 'text-neutral-400 hover:text-white'
            }`}
          >
            Sakugabooru
          </button>
        </div>

        {/* 内容区 */}
        {activeTab === 'local' ? (
          <>
            {/* 头部 */}
            <div className="p-4 border-b border-neutral-800">
              <h1 className="text-lg font-semibold tracking-tight">AAM</h1>
              <p className="text-xs text-neutral-500 mt-1">Animation Asset Manager</p>
            </div>

            {/* 导入按钮 */}
            <button
              onClick={handleImport}
              className="mx-4 mt-4 py-2 bg-white text-neutral-900 rounded-lg text-sm font-medium hover:bg-neutral-200 transition-colors"
            >
              + 导入本地文件
            </button>

            {/* 素材列表 */}
            <div className="flex-1 overflow-y-auto p-4">
              {isLoadingAssets ? (
                <div className="flex items-center justify-center h-20">
                  <div className="w-5 h-5 border-2 border-neutral-700 border-t-white rounded-full animate-spin" />
                </div>
              ) : assets.length === 0 ? (
                <p className="text-neutral-500 text-sm text-center mt-8">
                  还没有素材
                  <br />
                  导入文件或从Sakugabooru搜索
                </p>
              ) : (
                <div className="space-y-2">
                  {assets.map(asset => (
                    <button
                      key={asset.id}
                      onClick={() => selectAsset(asset)}
                      className={`w-full text-left p-3 rounded-lg transition-colors ${
                        selectedAsset?.id === asset.id
                          ? 'bg-white/10 border border-white/20'
                          : 'hover:bg-white/5 border border-transparent'
                      }`}
                    >
                      <div className="flex items-center gap-3">
                        <div className="w-12 h-12 bg-neutral-800 rounded flex items-center justify-center text-xs text-neutral-600">
                          {asset.format.toUpperCase()}
                        </div>
                        <div className="flex-1 min-w-0">
                          <p className="text-sm font-medium truncate">
                            {asset.id.slice(0, 8)}
                            {asset.source === 'sakugabooru' && (
                              <span className="ml-1 text-xs text-blue-400">[SB]</span>
                            )}
                          </p>
                          <p className="text-xs text-neutral-500">
                            {asset.frame_count} 帧 · {asset.fps} FPS
                          </p>
                        </div>
                      </div>
                    </button>
                  ))}
                </div>
              )}
            </div>
          </>
        ) : (
          <SakugaSearchPanel onImport={handleSakugaImport} />
        )}
      </aside>

      {/* 主视图区 */}
      <main className="flex-1 flex flex-col">
        {selectedAsset ? (
          <FrameViewer
            asset={selectedAsset}
            annotations={currentAnnotations}
            className="flex-1"
          />
        ) : (
          <div className="flex-1 flex items-center justify-center text-neutral-500">
            <div className="text-center">
              <p className="text-lg mb-2">选择一个素材查看</p>
              <p className="text-sm">或从左侧导入/搜索</p>
            </div>
          </div>
        )}
      </main>

      {/* 右侧属性面板 */}
      {selectedAsset && (
        <aside className="w-64 bg-neutral-900 border-l border-neutral-800 p-4 flex flex-col">
          <h2 className="text-sm font-medium mb-4">属性</h2>

          <div className="space-y-4 text-sm">
            <div>
              <p className="text-neutral-500">格式</p>
              <p className="font-medium">{selectedAsset.format.toUpperCase()}</p>
            </div>

            <div>
              <p className="text-neutral-500">分辨率</p>
              <p className="font-medium">
                {selectedAsset.resolution.width} × {selectedAsset.resolution.height}
              </p>
            </div>

            <div>
              <p className="text-neutral-500">时长</p>
              <p className="font-medium">
                {(selectedAsset.frame_count / selectedAsset.fps).toFixed(2)}秒
              </p>
            </div>

            <div>
              <p className="text-neutral-500">标注</p>
              <p className="font-medium">{currentAnnotations.length} 个</p>
            </div>

            {selectedAsset.source === 'sakugabooru' && selectedAsset.source_url && (
              <div>
                <p className="text-neutral-500">来源</p>
                <a
                  href={selectedAsset.source_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-blue-400 hover:underline text-xs break-all"
                >
                  Sakugabooru
                </a>
              </div>
            )}
          </div>

          {/* 导出选项 */}
          <div className="mt-6 space-y-2">
            <p className="text-xs text-neutral-500 uppercase tracking-wider">导出</p>

            <button
              onClick={() => handleExport('png_sequence')}
              className="w-full py-2 bg-neutral-800 hover:bg-neutral-700 rounded text-sm transition-colors"
            >
              PNG 序列帧
            </button>

            <button
              onClick={() => handleExport('h264')}
              className="w-full py-2 bg-neutral-800 hover:bg-neutral-700 rounded text-sm transition-colors"
            >
              H264 视频
            </button>

            <button
              onClick={() => handleExport('pro_res')}
              className="w-full py-2 bg-neutral-800 hover:bg-neutral-700 rounded text-sm transition-colors"
            >
              ProRes
            </button>

            <button
              onClick={() => handleExport('gif')}
              className="w-full py-2 bg-neutral-800 hover:bg-neutral-700 rounded text-sm transition-colors"
            >
              GIF
            </button>
          </div>

          {/* DCC 辅助脚本 */}
          <div className="mt-6 space-y-2">
            <p className="text-xs text-neutral-500 uppercase tracking-wider">导入脚本</p>

            <button
              onClick={() => handleGenerateScript('maya')}
              className="w-full py-2 bg-green-500/20 text-green-400 hover:bg-green-500/30 rounded text-sm transition-colors"
            >
              复制 Maya 脚本
            </button>

            <button
              onClick={() => handleGenerateScript('blender')}
              className="w-full py-2 bg-orange-500/20 text-orange-400 hover:bg-orange-500/30 rounded text-sm transition-colors"
            >
              复制 Blender 脚本
            </button>
          </div>

          {/* 删除按钮 */}
          <div className="mt-auto pt-4">
            <button
              onClick={() => {
                if (confirm('确定删除这个素材？')) {
                  useStore.getState().deleteAsset(selectedAsset.id);
                }
              }}
              className="w-full py-2 bg-red-500/20 text-red-400 hover:bg-red-500/30 rounded text-sm transition-colors"
            >
              删除素材
            </button>
          </div>
        </aside>
      )}
    </div>
  );
}

export default App;
