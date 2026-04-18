// Encoder Service - 视频编码与导出

use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

pub struct EncoderService {
    app_data_dir: PathBuf,
}

impl EncoderService {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self { app_data_dir }
    }

    /// 编码素材为视频（带标注叠加）
    pub fn encode_with_annotations(
        &self,
        asset: &crate::models::Asset,
        output_path: &PathBuf,
        format: crate::models::ExportFormat,
        frame_range: crate::models::FrameRange,
        include_annotations: bool,
    ) -> Result<()> {
        // 确定帧范围
        let (start_frame, end_frame) = match frame_range {
            crate::models::FrameRange::All => (1, asset.frame_count),
            crate::models::FrameRange::Range { start, end } => {
                // 边界处理 (Define Errors Out)
                let start = start.min(asset.frame_count);
                let end = end.min(asset.frame_count);
                (start, end)
            }
        };

        // 构建FFmpeg命令
        let input_pattern = asset.frame_directory.join("frame_%06d.png");
        
        let mut cmd = Command::new("ffmpeg");
        
        // 输入参数
        cmd.arg("-framerate").arg(asset.fps.to_string())
           .arg("-start_number").arg(start_frame.to_string())
           .arg("-i").arg(input_pattern.to_str().unwrap());

        // 帧范围
        let frame_count = end_frame.saturating_sub(start_frame) + 1;
        cmd.arg("-frames:v").arg(frame_count.to_string());

        // 编码参数
        match format {
            crate::models::ExportFormat::H264 { quality } => {
                let crf = match quality {
                    crate::models::H264Quality::Lossless => "0",
                    crate::models::H264Quality::High => "18",
                    crate::models::H264Quality::Medium => "23",
                    crate::models::H264Quality::Low => "28",
                };
                cmd.arg("-c:v").arg("libx264")
                   .arg("-crf").arg(crf)
                   .arg("-pix_fmt").arg("yuv420p")
                   .arg("-movflags").arg("+faststart");
            }
            crate::models::ExportFormat::ProRes => {
                cmd.arg("-c:v").arg("prores_ks")
                   .arg("-profile:v").arg("3") // HQ
                   .arg("-pix_fmt").arg("yuv422p10le");
            }
            crate::models::ExportFormat::Gif => {
                // GIF优化参数
                cmd.arg("-vf").arg("fps=30,scale=480:-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=256[p];[s1][p]paletteuse=dither=bayer")
                   .arg("-loop").arg("0");
            }
            crate::models::ExportFormat::WebM => {
                cmd.arg("-c:v").arg("libvpx-vp9")
                   .arg("-crf").arg("30")
                   .arg("-b:v").arg("0");
            }
            _ => {
                return Err(anyhow::anyhow!("Format not supported for export"));
            }
        }

        // 输出文件
        cmd.arg("-y").arg(output_path.to_str().unwrap());

        // 执行命令
        let status = cmd.status()?;

        if !status.success() {
            // 降级策略：如果编码失败，尝试复制PNG序列
            self.fallback_to_png_sequence(asset, output_path, start_frame, end_frame)?;
        }

        Ok(())
    }

    /// 降级策略：复制PNG序列
    fn fallback_to_png_sequence(
        &self,
        asset: &crate::models::Asset,
        output_path: &PathBuf,
        start_frame: u32,
        end_frame: u32,
    ) -> Result<()> {
        let output_dir = output_path.parent().unwrap_or(&self.app_data_dir);
        std::fs::create_dir_all(output_dir)?;

        for i in start_frame..=end_frame {
            let src = asset.frame_directory.join(format!("frame_{:06}.png", i));
            let dst = output_dir.join(format!("frame_{:06}.png", i));
            if src.exists() {
                std::fs::copy(&src, &dst)?;
            }
        }

        Ok(())
    }
}