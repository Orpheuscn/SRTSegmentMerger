use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
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
    
    pub fn seek(&mut self, position: f64) {
        // 停止当前播放
        if let Ok(sink) = self.sink.lock() {
            sink.stop();
        }
        
        // 创建新的 sink
        if let Ok(new_sink) = Sink::try_new(&self.stream_handle) {
            // 加载音频并跳到指定位置
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

