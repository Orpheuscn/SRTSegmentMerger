use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Result, anyhow};

/// 手动切割音频片段
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
    
    // 生成输出文件名
    let output_path = parent.join(format!("{}_manual_{:.2}_{:.2}.{}", 
        stem, start_time, end_time, extension));
    
    let duration = end_time - start_time;
    
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
        return Err(anyhow!("Failed to cut audio segment: {}", stderr));
    }
    
    Ok(output_path)
}

/// 解析时间字符串（支持 HH:MM:SS 或 MM:SS 或 SS）
pub fn parse_time_string(time_str: &str) -> Result<f64> {
    let parts: Vec<&str> = time_str.split(':').collect();
    
    let seconds = match parts.len() {
        1 => {
            // 只有秒
            parts[0].parse::<f64>()?
        }
        2 => {
            // MM:SS
            let minutes: f64 = parts[0].parse()?;
            let seconds: f64 = parts[1].parse()?;
            minutes * 60.0 + seconds
        }
        3 => {
            // HH:MM:SS
            let hours: f64 = parts[0].parse()?;
            let minutes: f64 = parts[1].parse()?;
            let seconds: f64 = parts[2].parse()?;
            hours * 3600.0 + minutes * 60.0 + seconds
        }
        _ => return Err(anyhow!("Invalid time format"))
    };
    
    Ok(seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_time_string() {
        assert_eq!(parse_time_string("30").unwrap(), 30.0);
        assert_eq!(parse_time_string("1:30").unwrap(), 90.0);
        assert_eq!(parse_time_string("1:30:45").unwrap(), 5445.0);
    }
}

