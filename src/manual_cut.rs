use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Result, anyhow};
use crate::ffmpeg;

/// 手动切割音频片段
/// 
/// 注意：切割后会将 WAV 片段转换为 MP3 格式，并删除 WAV 片段
pub fn cut_audio_segment(
    audio_path: &Path,
    start_time: f64,
    end_time: f64,
) -> Result<PathBuf> {
    if start_time >= end_time {
        return Err(anyhow!("Start time must be less than end time"));
    }
    
    let parent = audio_path.parent().unwrap();
    let stem = audio_path.file_stem().unwrap().to_string_lossy();
    let extension = audio_path.extension().unwrap().to_string_lossy();
    
    // 生成 WAV 输出文件名（临时）
    let wav_output_path = parent.join(format!("{}_manual_{:.2}_{:.2}.{}", 
        stem, start_time, end_time, extension));
    
    let duration = end_time - start_time;
    
    println!("🔪 手动切割音频片段 ({:.2}s - {:.2}s)...", start_time, end_time);
    
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
        .arg(&wav_output_path)
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to cut audio segment: {}", stderr));
    }
    
    // 转换为 MP3
    println!("🎵 转换片段为 MP3 格式...");
    let mp3_path = ffmpeg::convert_wav_to_mp3(&wav_output_path)?;
    println!("✅ 手动切割完成: {:?}", mp3_path);
    
    Ok(mp3_path)
}
