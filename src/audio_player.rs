use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::process::Command;
use anyhow::Result;

pub struct AudioPlayer {
    audio_path: PathBuf,
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sink: Arc<Mutex<Sink>>,
    duration: f64,
    start_time: Arc<Mutex<std::time::Instant>>,
    paused_at: Arc<Mutex<Option<f64>>>,
    is_playing: Arc<Mutex<bool>>,
    temp_seek_file: Arc<Mutex<Option<PathBuf>>>,  // 临时seek文件路径
}

impl AudioPlayer {
    pub fn new(path: &Path) -> Result<Self> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        
        // 加载音频文件获取时长
        let file = File::open(path)?;
        let source = Decoder::new(BufReader::new(file))?;
        let duration = source.total_duration()
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        
        // 重新加载音频用于播放
        let file = File::open(path)?;
        let source = Decoder::new(BufReader::new(file))?;
        sink.append(source);
        sink.pause();
        
        Ok(AudioPlayer {
            audio_path: path.to_path_buf(),
            _stream,
            stream_handle,
            sink: Arc::new(Mutex::new(sink)),
            duration,
            start_time: Arc::new(Mutex::new(std::time::Instant::now())),
            paused_at: Arc::new(Mutex::new(Some(0.0))),
            is_playing: Arc::new(Mutex::new(false)),
            temp_seek_file: Arc::new(Mutex::new(None)),
        })
    }
    
    pub fn play(&mut self) {
        if let Ok(sink) = self.sink.lock() {
            if sink.empty() {
                // 如果 sink 为空（可能因为 seek 操作），重新加载
                if let Ok(file) = File::open(&self.audio_path) {
                    if let Ok(source) = Decoder::new(BufReader::new(file)) {
                        let current_pos = self.paused_at.lock().unwrap().unwrap_or(0.0);
                        // 跳过前面的部分
                        let source = source.skip_duration(Duration::from_secs_f64(current_pos));
                        sink.append(source);
                    }
                }
            }
            
            sink.play();
            
            // 更新开始时间
            let paused_position = self.paused_at.lock().unwrap().unwrap_or(0.0);
            *self.start_time.lock().unwrap() = std::time::Instant::now() - Duration::from_secs_f64(paused_position);
            *self.paused_at.lock().unwrap() = None;
            *self.is_playing.lock().unwrap() = true;
        }
    }
    
    pub fn pause(&mut self) {
        if let Ok(sink) = self.sink.lock() {
            sink.pause();
            
            // 记录暂停位置
            let current_pos = self.position();
            *self.paused_at.lock().unwrap() = Some(current_pos);
            *self.is_playing.lock().unwrap() = false;
        }
    }
    
    /// 使用FFmpeg创建快速seek文件
    /// 这样可以避免rodio的skip_duration性能问题
    fn create_seek_segment(&self, position: f64) -> Result<PathBuf> {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("whisper_seek_{}.wav", 
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()));
        
        // 使用FFmpeg从目标位置开始提取音频
        // 只提取接下来的一段（比如30秒），这样文件更小，加载更快
        let duration_to_extract = (self.duration - position).min(30.0);
        
        let output = Command::new("ffmpeg")
            .arg("-ss")
            .arg(position.to_string())
            .arg("-i")
            .arg(&self.audio_path)
            .arg("-t")
            .arg(duration_to_extract.to_string())
            .arg("-acodec")
            .arg("pcm_s16le")
            .arg("-ar")
            .arg("44100")
            .arg("-ac")
            .arg("2")
            .arg("-y")
            .arg(&temp_file)
            .output()?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("FFmpeg seek failed"));
        }
        
        Ok(temp_file)
    }
    
    /// 清理旧的临时seek文件
    fn cleanup_temp_seek_file(&self) {
        if let Ok(mut temp_file) = self.temp_seek_file.lock() {
            if let Some(path) = temp_file.take() {
                let _ = fs::remove_file(path);
            }
        }
    }
    
    pub fn seek(&mut self, position: f64) {
        // 限制position在有效范围内
        let position = position.max(0.0).min(self.duration);
        
        // 停止当前播放
        if let Ok(sink) = self.sink.lock() {
            sink.stop();
        }
        
        // 创建新的 sink
        if let Ok(new_sink) = Sink::try_new(&self.stream_handle) {
            // 对于接近开头的位置，直接使用原文件
            if position < 1.0 {
                if let Ok(file) = File::open(&self.audio_path) {
                    if let Ok(source) = Decoder::new(BufReader::new(file)) {
                        let source = source.skip_duration(Duration::from_secs_f64(position));
                        new_sink.append(source);
                        
                        let was_playing = *self.is_playing.lock().unwrap();
                        if was_playing {
                            new_sink.play();
                            *self.start_time.lock().unwrap() = std::time::Instant::now() - Duration::from_secs_f64(position);
                            *self.paused_at.lock().unwrap() = None;
                        } else {
                            new_sink.pause();
                            *self.paused_at.lock().unwrap() = Some(position);
                        }
                        
                        *self.sink.lock().unwrap() = new_sink;
                    }
                }
            } else {
                // 对于较大的seek，使用FFmpeg预先处理
                // 这样可以避免rodio的skip_duration性能问题
                match self.create_seek_segment(position) {
                    Ok(seek_file) => {
                        // 先清理旧的临时文件
                        self.cleanup_temp_seek_file();
                        
                        if let Ok(file) = File::open(&seek_file) {
                            if let Ok(source) = Decoder::new(BufReader::new(file)) {
                                new_sink.append(source);
                                
                                let was_playing = *self.is_playing.lock().unwrap();
                                if was_playing {
                                    new_sink.play();
                                    *self.start_time.lock().unwrap() = std::time::Instant::now() - Duration::from_secs_f64(position);
                                    *self.paused_at.lock().unwrap() = None;
                                } else {
                                    new_sink.pause();
                                    *self.paused_at.lock().unwrap() = Some(position);
                                }
                                
                                *self.sink.lock().unwrap() = new_sink;
                                
                                // 保存临时文件路径以便后续清理
                                *self.temp_seek_file.lock().unwrap() = Some(seek_file);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("快速seek失败，回退到慢速模式: {}", e);
                        // 如果FFmpeg失败，回退到原来的方法
                        if let Ok(file) = File::open(&self.audio_path) {
                            if let Ok(source) = Decoder::new(BufReader::new(file)) {
                                let source = source.skip_duration(Duration::from_secs_f64(position));
                                new_sink.append(source);
                                
                                let was_playing = *self.is_playing.lock().unwrap();
                                if was_playing {
                                    new_sink.play();
                                    *self.start_time.lock().unwrap() = std::time::Instant::now() - Duration::from_secs_f64(position);
                                    *self.paused_at.lock().unwrap() = None;
                                } else {
                                    new_sink.pause();
                                    *self.paused_at.lock().unwrap() = Some(position);
                                }
                                
                                *self.sink.lock().unwrap() = new_sink;
                            }
                        }
                    }
                }
            }
        }
    }
    
    pub fn position(&self) -> f64 {
        if let Some(paused) = *self.paused_at.lock().unwrap() {
            paused
        } else {
            let elapsed = self.start_time.lock().unwrap().elapsed().as_secs_f64();
            elapsed.min(self.duration)
        }
    }
    
    pub fn duration(&self) -> f64 {
        self.duration
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        // 清理临时seek文件
        self.cleanup_temp_seek_file();
    }
}

