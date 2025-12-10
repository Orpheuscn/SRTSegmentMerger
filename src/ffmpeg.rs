use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
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

/// 将 WAV 音频文件转换为 MP3 格式
/// 
/// 参数：
/// - wav_path: WAV 文件路径
/// 
/// 返回：MP3 文件路径
/// 
/// 注意：转换完成后会删除原始 WAV 文件
pub fn convert_wav_to_mp3(wav_path: &Path) -> Result<PathBuf> {
    let mp3_path = wav_path.with_extension("mp3");
    
    // 使用 ffmpeg 转换为 MP3
    // 使用较高的比特率以保证质量
    let output = Command::new("ffmpeg")
        .arg("-i")
        .arg(wav_path)
        .arg("-codec:a")
        .arg("libmp3lame")
        .arg("-b:a")
        .arg("192k")  // 192 kbps 比特率，平衡质量和文件大小
        .arg("-y")
        .arg(&mp3_path)
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("转换为 MP3 失败: {}", stderr));
    }
    
    // 验证 MP3 文件是否生成成功
    if !mp3_path.exists() {
        return Err(anyhow!("MP3 文件未生成"));
    }
    
    // 删除原始 WAV 文件
    if let Err(e) = fs::remove_file(wav_path) {
        eprintln!("警告: 删除 WAV 文件失败: {}", e);
        // 不返回错误，因为 MP3 已经生成成功
    }
    
    Ok(mp3_path)
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

