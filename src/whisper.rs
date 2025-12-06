use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::fs;
use std::io::{BufRead, BufReader};
use anyhow::{Result, anyhow};
use crate::{WhisperModel, ProgressMessage};
use std::sync::mpsc::Sender;

/// 使用 Whisper 识别音频（保留用于兼容性）
#[allow(dead_code)]
pub fn recognize_audio(
    audio_path: &Path,
    model: WhisperModel,
    language: Option<&str>,
) -> Result<(PathBuf, String)> {
    let output_dir = audio_path.parent().unwrap();
    let output_name = audio_path.file_stem().unwrap().to_string_lossy();
    
    let mut cmd = Command::new("whisper");
    
    cmd.arg(audio_path)
        .arg("--model")
        .arg(model.as_str())
        .arg("--output_format")
        .arg("srt")
        .arg("--output_dir")
        .arg(output_dir);
    
    // 如果指定了语言，添加语言参数
    if let Some(lang) = language {
        cmd.arg("--language").arg(lang);
    }
    
    let output = cmd.output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Whisper recognition failed: {}", stderr));
    }
    
    // Whisper 输出的 SRT 文件名
    let srt_path = output_dir.join(format!("{}.srt", output_name));
    
    if !srt_path.exists() {
        return Err(anyhow!("Subtitle file not found"));
    }
    
    // 读取并提取文本内容
    let text = extract_text_from_srt(&srt_path)?;
    
    Ok((srt_path, text))
}

/// 使用 Whisper 识别音频（实时输出版本）
pub fn recognize_audio_realtime(
    audio_path: &Path,
    model: WhisperModel,
    language: Option<&str>,
    tx: Sender<ProgressMessage>,
    current: usize,
    total: usize,
) -> Result<(PathBuf, String)> {
    let output_dir = audio_path.parent().unwrap();
    let output_name = audio_path.file_stem().unwrap().to_string_lossy();
    
    let mut cmd = Command::new("whisper");
    
    cmd.arg(audio_path)
        .arg("--model")
        .arg(model.as_str())
        .arg("--output_format")
        .arg("srt")
        .arg("--output_dir")
        .arg(output_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    // 如果指定了语言，添加语言参数
    if let Some(lang) = language {
        cmd.arg("--language").arg(lang);
    }
    
    let mut child = cmd.spawn()?;
    
    // 读取 stderr（Whisper 将进度输出到 stderr）
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                // 只发送包含有用信息的行
                if !line.trim().is_empty() && (line.contains("[") || line.contains("Detecting language")) {
                    let msg = format!("[{}/{}] {}", current, total, line.trim());
                    let _ = tx.send(ProgressMessage::RealtimeOutput(msg));
                }
            }
        }
    }
    
    let status = child.wait()?;
    
    if !status.success() {
        return Err(anyhow!("Whisper recognition failed"));
    }
    
    // Whisper 输出的 SRT 文件名
    let srt_path = output_dir.join(format!("{}.srt", output_name));
    
    if !srt_path.exists() {
        return Err(anyhow!("Subtitle file not found"));
    }
    
    // 读取并提取文本内容
    let text = extract_text_from_srt(&srt_path)?;
    
    Ok((srt_path, text))
}

/// 从 SRT 文件中提取纯文本
fn extract_text_from_srt(srt_path: &Path) -> Result<String> {
    let content = fs::read_to_string(srt_path)?;
    let mut text_lines = Vec::new();
    
    for line in content.lines() {
        let line = line.trim();
        // 跳过序号行、时间轴行和空行
        if line.is_empty() 
            || line.parse::<u32>().is_ok() 
            || line.contains("-->") {
            continue;
        }
        text_lines.push(line);
    }
    
    Ok(text_lines.join(" "))
}

