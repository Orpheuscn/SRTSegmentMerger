use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Result, anyhow};

/// 使用 FFmpeg 检测并提取音频
pub fn extract_audio(video_path: &Path) -> Result<PathBuf> {
    // 直接转换为 WAV 格式以确保最大兼容性
    let wav_path = video_path.with_extension("wav");
    
    let output = Command::new("ffmpeg")
        .arg("-i")
        .arg(video_path)
        .arg("-vn")            // 不处理视频
        .arg("-acodec")
        .arg("pcm_s16le")      // 转换为 WAV PCM 16-bit
        .arg("-ar")
        .arg("44100")          // 采样率 44.1kHz (标准音质)
        .arg("-ac")
        .arg("2")              // 立体声
        .arg("-y")             // 覆盖输出文件
        .arg(&wav_path)
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("FFmpeg failed to extract audio: {}", stderr));
    }
    
    Ok(wav_path)
}

/// 根据切割点切割音频文件
pub fn cut_audio(audio_path: &Path, cut_points: &[f64]) -> Result<Vec<PathBuf>> {
    if cut_points.is_empty() {
        // 如果没有切割点，返回原始文件
        return Ok(vec![audio_path.to_path_buf()]);
    }
    
    let mut segments = Vec::new();
    let mut start_time = 0.0;
    
    // 创建输出目录
    let parent = audio_path.parent().unwrap();
    let stem = audio_path.file_stem().unwrap().to_string_lossy();
    let extension = audio_path.extension().unwrap().to_string_lossy();
    
    // 根据切割点生成片段
    for (i, &cut_point) in cut_points.iter().enumerate() {
        let output_path = parent.join(format!("{}_{:03}.{}", stem, i, extension));
        
        let duration = cut_point - start_time;
        
        let output = Command::new("ffmpeg")
            .arg("-i")
            .arg(audio_path)
            .arg("-ss")
            .arg(start_time.to_string())
            .arg("-t")
            .arg(duration.to_string())
            .arg("-acodec")
            .arg("copy")
            .arg("-y")
            .arg(&output_path)
            .output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("切割音频失败: {}", stderr));
        }
        
        segments.push(output_path);
        start_time = cut_point;
    }
    
    // 最后一段：从最后一个切割点到结束
    let output_path = parent.join(format!("{}_{:03}.{}", stem, cut_points.len(), extension));
    
    let output = Command::new("ffmpeg")
        .arg("-i")
        .arg(audio_path)
        .arg("-ss")
        .arg(start_time.to_string())
        .arg("-acodec")
        .arg("copy")
        .arg("-y")
        .arg(&output_path)
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("切割最后一段音频失败: {}", stderr));
    }
    
    segments.push(output_path);
    
    Ok(segments)
}

/// 获取音频文件的时长
#[allow(dead_code)]
fn get_audio_duration(audio_path: &Path) -> Result<f64> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(audio_path)
        .output()?;
    
    if !output.status.success() {
        return Err(anyhow!("获取音频时长失败"));
    }
    
    let duration_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let duration: f64 = duration_str.parse()?;
    
    Ok(duration)
}

