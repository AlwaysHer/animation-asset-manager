// Decoder Service - GIF/视频解码

use anyhow::Result;
use std::path::PathBuf;
use crate::models::{Asset, Frame, AssetFormat};

pub struct DecoderService {
    frames_base_dir: PathBuf,
}

impl DecoderService {
    pub fn new(frames_base_dir: PathBuf) -> Self {
        Self { frames_base_dir }
    }

    /// 解码素材为帧序列
    pub fn decode_asset(&self, asset: &Asset) -> Result<Vec<Frame>> {
        // 确保帧目录存在
        std::fs::create_dir_all(&asset.frame_directory)?;

        match asset.format {
            AssetFormat::Gif => self.decode_gif(asset),
            AssetFormat::Webm | AssetFormat::Mp4 | AssetFormat::Mov => {
                self.decode_video(asset)
            }
            _ => Err(anyhow::anyhow!("Unsupported format: {:?}", asset.format)),
        }
    }

    fn decode_gif(&self, asset: &Asset) -> Result<Vec<Frame>> {
        use std::fs::File;
        use gif::DecodeOptions;

        let file = File::open(&asset.original_path)?;
        let mut options = DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);

        let mut decoder = options.read_info(file)?;
        let mut frames = Vec::new();
        let mut frame_index = 0u32;

        while let Some(frame) = decoder.read_next_frame()? {
            let filename = format!("frame_{:06}.png", frame_index);
            let frame_path = asset.frame_directory.join(&filename);

            // 保存为PNG
            let img = image::RgbaImage::from_raw(
                frame.width as u32,
                frame.height as u32,
                frame.buffer.to_vec()
            ).ok_or_else(|| anyhow::anyhow!("Failed to create image buffer"))?;

            img.save_with_format(&frame_path, image::ImageFormat::Png)?;

            frames.push(Frame {
                asset_id: asset.id.clone(),
                index: frame_index,
                timestamp_ms: frame.delay as u64 * 10, // GIF延迟单位是1/100秒
                filename,
                has_annotation: false,
            });

            frame_index += 1;
        }

        Ok(frames)
    }

    fn decode_video(&self, asset: &Asset) -> Result<Vec<Frame>> {
        // 使用FFmpeg解码视频
        let output_pattern = asset.frame_directory.join("frame_%06d.png");
        
        let status = std::process::Command::new("ffmpeg")
            .args(&[
                "-i", asset.original_path.to_str().unwrap(),
                "-vf", "fps=30,scale=trunc(iw/2)*2:trunc(ih/2)*2",
                "-pix_fmt", "rgba",
                output_pattern.to_str().unwrap()
            ])
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("FFmpeg failed to decode video"));
        }

        // 扫描生成的帧
        let mut frames = Vec::new();
        let entries = std::fs::read_dir(&asset.frame_directory)?;
        
        for (i, entry) in entries.filter_map(|e| e.ok()).enumerate() {
            let filename = entry.file_name().to_string_lossy().to_string();
            if filename.ends_with(".png") {
                frames.push(Frame {
                    asset_id: asset.id.clone(),
                    index: i as u32,
                    timestamp_ms: (i as u64 * 1000) / 30, // 假设30fps
                    filename,
                    has_annotation: false,
                });
            }
        }

        frames.sort_by(|a, b| a.index.cmp(&b.index));
        Ok(frames)
    }
}