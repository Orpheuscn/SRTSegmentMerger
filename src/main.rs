mod audio_player;
mod ffmpeg;
mod whisper;
mod srt_merger;
mod recognition;
mod manual_cut;

use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::process::Command;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 700.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "SRT Segment Merger",
        options,
        Box::new(|_cc| Ok(Box::new(WhisperApp::default()))),
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AppState {
    Idle,
    AudioExtracted,
    Processing,
}

#[derive(Default)]
struct WhisperApp {
    // 文件路径
    video_path: Option<PathBuf>,
    audio_path: Option<PathBuf>,
    
    // 应用状态
    state: AppState,
    status_message: String,
    
    // 音频播放器
    audio_player: Option<audio_player::AudioPlayer>,
    is_playing: bool,
    current_position: f64, // 秒
    total_duration: f64,   // 秒
    
    // Whisper 参数
    whisper_model: WhisperModel,
    whisper_language: WhisperLanguage,
    custom_language_code: String,
    
    // 切割后的音频文件
    audio_segments: Vec<PathBuf>,
    
    // 进度信息
    processing_progress: f32,
    processing_status: String,
    
    // 识别结果
    recognition_results: Vec<String>,
    
    // 消息通道
    progress_receiver: Option<Receiver<ProgressMessage>>,
    
    // 手动切割
    manual_start_hour: String,
    manual_start_minute: String,
    manual_start_second: String,
    manual_start_millisecond: String,
    manual_end_hour: String,
    manual_end_minute: String,
    manual_end_second: String,
    manual_end_millisecond: String,
    manual_segment: Option<PathBuf>,
    
    // 完整字幕
    complete_srt_path: String,
    complete_srt_loaded: bool,
    
    // 片段字幕
    segment_srt_path: String,
    segment_srt_loaded: bool,
}

#[derive(Debug, Clone)]
enum ProgressMessage {
    Progress { current: usize, total: usize },
    Result { segment: usize, text: String },
    RealtimeOutput(String),  // 实时输出信息
    Completed,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum WhisperModel {
    Tiny,
    Base,
    Small,
    Medium,
    Large,
    Turbo,
}

impl Default for WhisperModel {
    fn default() -> Self {
        WhisperModel::Base
    }
}

impl WhisperModel {
    fn as_str(&self) -> &str {
        match self {
            WhisperModel::Tiny => "tiny",
            WhisperModel::Base => "base",
            WhisperModel::Small => "small",
            WhisperModel::Medium => "medium",
            WhisperModel::Large => "large",
            WhisperModel::Turbo => "turbo",
        }
    }
    
    fn all() -> Vec<WhisperModel> {
        vec![
            WhisperModel::Tiny,
            WhisperModel::Base,
            WhisperModel::Small,
            WhisperModel::Medium,
            WhisperModel::Large,
            WhisperModel::Turbo,
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
enum WhisperLanguage {
    Unknown,
    Japanese,
    English,
    Chinese,
    French,
    German,
    Spanish,
    Italian,
    Russian,
    Custom,
}

impl Default for WhisperLanguage {
    fn default() -> Self {
        WhisperLanguage::Unknown
    }
}

impl WhisperLanguage {
    fn as_str(&self) -> &str {
        match self {
            WhisperLanguage::Unknown => "Auto Detect",
            WhisperLanguage::Japanese => "Japanese",
            WhisperLanguage::English => "English",
            WhisperLanguage::Chinese => "Chinese",
            WhisperLanguage::French => "French",
            WhisperLanguage::German => "German",
            WhisperLanguage::Spanish => "Spanish",
            WhisperLanguage::Italian => "Italian",
            WhisperLanguage::Russian => "Russian",
            WhisperLanguage::Custom => "Custom (Manual Input)",
        }
    }
    
    fn all() -> Vec<WhisperLanguage> {
        vec![
            WhisperLanguage::Unknown,
            WhisperLanguage::English,
            WhisperLanguage::Chinese,
            WhisperLanguage::Japanese,
            WhisperLanguage::French,
            WhisperLanguage::German,
            WhisperLanguage::Spanish,
            WhisperLanguage::Italian,
            WhisperLanguage::Russian,
            WhisperLanguage::Custom,
        ]
    }
}

impl Default for AppState {
    fn default() -> Self {
        AppState::Idle
    }
}

impl WhisperApp {
    fn handle_dropped_file(&mut self, path: PathBuf) {
        self.video_path = Some(path.clone());
        self.state = AppState::Idle;
        self.status_message = format!("File loaded: {:?}", path.file_name().unwrap());
        self.audio_path = None;
        self.audio_player = None;
        self.audio_segments.clear();
        self.recognition_results.clear();
        
        // 检查文件类型：如果是音频文件，直接使用；如果是视频，提取音频
        let extension = path.extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        
        if matches!(extension.as_str(), "wav" | "mp3" | "m4a" | "flac" | "ogg" | "opus") {
            // 直接使用音频文件
            self.load_audio_file(path);
        } else {
            // 从视频中提取音频
            self.extract_audio();
        }
    }
    
    fn load_audio_file(&mut self, audio_path: PathBuf) {
        self.audio_path = Some(audio_path.clone());
        self.status_message = "Audio file loaded!".to_string();
        self.state = AppState::AudioExtracted;
        
        // 加载音频播放器
        match audio_player::AudioPlayer::new(&audio_path) {
            Ok(player) => {
                self.total_duration = player.duration();
                self.audio_player = Some(player);
            }
            Err(e) => {
                self.status_message = format!("Failed to load audio: {}", e);
            }
        }
    }
    
    fn extract_audio(&mut self) {
        if let Some(video_path) = &self.video_path {
            self.status_message = "Extracting audio...".to_string();
            
            match ffmpeg::extract_audio(video_path) {
                Ok(audio_path) => {
                    self.audio_path = Some(audio_path.clone());
                    self.status_message = "Audio extracted successfully!".to_string();
                    self.state = AppState::AudioExtracted;
                    
                    // Load audio player
                    match audio_player::AudioPlayer::new(&audio_path) {
                        Ok(player) => {
                            self.total_duration = player.duration();
                            self.audio_player = Some(player);
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to load audio: {}", e);
                        }
                    }
                }
                Err(e) => {
                    self.status_message = format!("Failed to extract audio: {}", e);
                }
            }
        }
    }
    
    fn start_recognition(&mut self) {
        if self.audio_segments.is_empty() {
            self.status_message = "Please cut audio first!".to_string();
            return;
        }
        
        self.state = AppState::Processing;
        self.processing_progress = 0.0;
        self.processing_status = "Starting recognition...".to_string();
        self.recognition_results.clear();
        
        let segments = self.audio_segments.clone();
        let model = self.whisper_model;
        let language = self.whisper_language.clone();
        let custom_lang = self.custom_language_code.clone();
        
        // 创建消息通道
        let (tx, rx) = channel();
        self.progress_receiver = Some(rx);
        
        std::thread::spawn(move || {
            let total = segments.len();
            let mut srt_files = Vec::new();
            
            for (i, segment) in segments.iter().enumerate() {
                // 确定要使用的语言代码
                let lang_code = match language {
                    WhisperLanguage::Unknown => None,
                    WhisperLanguage::Japanese => Some("ja"),
                    WhisperLanguage::English => Some("en"),
                    WhisperLanguage::Chinese => Some("zh"),
                    WhisperLanguage::French => Some("fr"),
                    WhisperLanguage::German => Some("de"),
                    WhisperLanguage::Spanish => Some("es"),
                    WhisperLanguage::Italian => Some("it"),
                    WhisperLanguage::Russian => Some("ru"),
                    WhisperLanguage::Custom => {
                        if custom_lang.is_empty() {
                            None
                        } else {
                            Some(custom_lang.as_str())
                        }
                    }
                };
                
                // 使用新的实时输出版本
                match whisper::recognize_audio_realtime(segment, model, lang_code, tx.clone(), i + 1, total) {
                    Ok((srt_path, text)) => {
                        srt_files.push(srt_path);
                        // 发送识别结果
                        let _ = tx.send(ProgressMessage::Result { 
                            segment: i + 1, 
                            text 
                        });
                        // 发送进度（识别完成后）
                        let _ = tx.send(ProgressMessage::Progress { 
                            current: i + 1, 
                            total 
                        });
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to recognize segment {}: {}", i + 1, e);
                        eprintln!("{}", error_msg);
                        let _ = tx.send(ProgressMessage::Error(error_msg));
                    }
                }
            }
            
            // Note: No auto-merge for manual segment workflow
            
            // 发送完成消息
            let _ = tx.send(ProgressMessage::Completed);
        });
    }
    
    fn format_time(seconds: f64) -> String {
        let hours = (seconds / 3600.0).floor() as u32;
        let minutes = ((seconds % 3600.0) / 60.0).floor() as u32;
        let secs = (seconds % 60.0).floor() as u32;
        let millis = ((seconds % 1.0) * 1000.0).floor() as u32;
        
        format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, secs, millis)
    }
    
    fn cut_manual_segment(&mut self) {
        if let Some(audio_path) = &self.audio_path {
            // 解析时间
            let start_time = self.parse_manual_time(
                &self.manual_start_hour,
                &self.manual_start_minute,
                &self.manual_start_second,
                &self.manual_start_millisecond,
            );
            
            let end_time = self.parse_manual_time(
                &self.manual_end_hour,
                &self.manual_end_minute,
                &self.manual_end_second,
                &self.manual_end_millisecond,
            );
            
            match (start_time, end_time) {
                (Ok(start), Ok(end)) => {
                    // 切割片段
                    match manual_cut::cut_audio_segment(audio_path, start, end) {
                        Ok(segment_path) => {
                            self.manual_segment = Some(segment_path);
                            self.status_message = format!("Manual segment cut: {:.3}s - {:.3}s", start, end);
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to cut segment: {}", e);
                        }
                    }
                }
                (Err(_), _) => {
                    self.status_message = "Invalid start time!".to_string();
                }
                (_, Err(_)) => {
                    self.status_message = "Invalid end time!".to_string();
                }
            }
        }
    }
    
    fn parse_manual_time(&self, hour: &str, minute: &str, second: &str, millisecond: &str) -> Result<f64, ()> {
        let h: f64 = if hour.is_empty() { 0.0 } else { hour.parse().map_err(|_| ())? };
        let m: f64 = if minute.is_empty() { 0.0 } else { minute.parse().map_err(|_| ())? };
        let s: f64 = if second.is_empty() { 0.0 } else { second.parse().map_err(|_| ())? };
        let ms: f64 = if millisecond.is_empty() { 0.0 } else { millisecond.parse().map_err(|_| ())? };
        
        Ok(h * 3600.0 + m * 60.0 + s + ms / 1000.0)
    }
    
    fn load_srt_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("SRT", &["srt"])
            .pick_file()
        {
            self.complete_srt_path = path.to_string_lossy().to_string();
            self.complete_srt_loaded = true;
            self.status_message = format!("Complete SRT loaded: {}", path.file_name().unwrap().to_string_lossy());
        }
    }
    
    fn load_segment_srt_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("SRT", &["srt"])
            .pick_file()
        {
            self.segment_srt_path = path.to_string_lossy().to_string();
            self.segment_srt_loaded = true;
            self.status_message = format!("Segment SRT loaded: {}", path.file_name().unwrap().to_string_lossy());
        }
    }
    
    fn merge_segment_subtitle(&mut self) {
        if self.complete_srt_path.is_empty() {
            self.status_message = "Please load complete SRT file first!".to_string();
            return;
        }
        
        // Check if we have a segment subtitle to merge
        let segment_srt = if !self.segment_srt_path.is_empty() {
            // User manually loaded a segment SRT
            PathBuf::from(&self.segment_srt_path)
        } else if let Some(ref manual_seg) = self.manual_segment {
            // Use recognized segment SRT
            manual_seg.with_extension("srt")
        } else {
            self.status_message = "No segment subtitle to merge!".to_string();
            return;
        };
        
        if !segment_srt.exists() {
            self.status_message = "Segment SRT file not found!".to_string();
            return;
        }
        
        // Get segment start time
        let start_time = match self.parse_manual_time(
            &self.manual_start_hour,
            &self.manual_start_minute,
            &self.manual_start_second,
            &self.manual_start_millisecond,
        ) {
            Ok(t) => t,
            Err(_) => {
                self.status_message = "Invalid start time!".to_string();
                return;
            }
        };
        
        let complete_srt = PathBuf::from(&self.complete_srt_path);
        
        // Directly replace the source file
        match srt_merger::insert_segment_subtitle(&complete_srt, &segment_srt, start_time, &complete_srt) {
            Ok(_) => {
                self.status_message = format!("Merged! Updated: {}", complete_srt.file_name().unwrap().to_string_lossy());
            }
            Err(e) => {
                self.status_message = format!("Merge failed: {}", e);
            }
        }
    }
    
    fn recognize_manual_segment(&mut self) {
        if self.manual_segment.is_none() {
            self.status_message = "No manual segment to recognize!".to_string();
            return;
        }
        
        self.state = AppState::Processing;
        self.processing_progress = 0.0;
        self.processing_status = "Recognizing manual segment...".to_string();
        self.recognition_results.clear();
        
        let segment = self.manual_segment.clone().unwrap();
        let model = self.whisper_model;
        let language = self.whisper_language.clone();
        let custom_lang = self.custom_language_code.clone();
        
        // 创建消息通道
        let (tx, rx) = channel();
        self.progress_receiver = Some(rx);
        
        std::thread::spawn(move || {
            // 识别手动片段
            match recognition::recognize_single_segment(
                &segment,
                0,
                1,
                model,
                &language,
                &custom_lang,
                tx.clone(),
            ) {
                Ok((_srt_path, text)) => {
                    let _ = tx.send(ProgressMessage::Result { 
                        segment: 0, 
                        text 
                    });
                    let _ = tx.send(ProgressMessage::Progress { 
                        current: 1, 
                        total: 1 
                    });
                    
                    // Segment recognized successfully
                    // User needs to manually click "Merge" button to insert into complete subtitle
                }
                Err(e) => {
                    let error_msg = format!("Failed to recognize manual segment: {}", e);
                    eprintln!("{}", error_msg);
                    let _ = tx.send(ProgressMessage::Error(error_msg));
                }
            }
            
            let _ = tx.send(ProgressMessage::Completed);
        });
    }
    
    fn stop_recognition(&mut self) {
        // 终止所有 whisper 和 python 进程
        Self::kill_whisper_processes();
        
        // 重置状态
        self.state = AppState::AudioExtracted;
        self.status_message = "Recognition stopped and all processes killed.".to_string();
        self.progress_receiver = None;
        self.processing_progress = 0.0;
        self.processing_status = String::new();
    }
    
    fn kill_whisper_processes() {
        // 查找并终止所有 whisper 相关进程
        if let Ok(output) = Command::new("ps")
            .args(&["aux"])
            .output()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            
            for line in output_str.lines() {
                // 查找包含 whisper 的进程
                if line.contains("whisper") && !line.contains("grep") {
                    if let Some(pid) = Self::extract_pid_from_ps_line(line) {
                        let _ = Command::new("kill")
                            .args(&["-9", &pid.to_string()])
                            .output();
                    }
                }
                
                // 查找包含 python 且包含 whisper 的进程
                if line.contains("python") && line.contains("whisper") && !line.contains("grep") {
                    if let Some(pid) = Self::extract_pid_from_ps_line(line) {
                        let _ = Command::new("kill")
                            .args(&["-9", &pid.to_string()])
                            .output();
                    }
                }
            }
        }
    }
    
    fn extract_pid_from_ps_line(line: &str) -> Option<u32> {
        // ps aux 输出格式：USER PID ...
        // 提取第二列（PID）
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            parts[1].parse::<u32>().ok()
        } else {
            None
        }
    }
    
    
}

impl eframe::App for WhisperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 处理识别进度消息
        let mut should_complete = false;
        if let Some(rx) = &self.progress_receiver {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ProgressMessage::Progress { current, total } => {
                        self.processing_status = format!("Recognizing segment {}/{}", current, total);
                        self.processing_progress = current as f32 / total as f32;
                    }
                    ProgressMessage::Result { segment, text } => {
                        let result = format!("\n=== Segment {} Recognized ===\n{}\n", segment, text);
                        self.recognition_results.push(result);
                    }
                    ProgressMessage::RealtimeOutput(output) => {
                        // Whisper real-time log output
                        self.recognition_results.push(output);
                    }
                    ProgressMessage::Completed => {
                        should_complete = true;
                    }
                    ProgressMessage::Error(err) => {
                        self.recognition_results.push(format!("❌ Error: {}", err));
                    }
                }
            }
        }
        
        if should_complete {
            self.state = AppState::AudioExtracted;
            self.status_message = "Recognition completed!".to_string();
            self.progress_receiver = None;
        }
        
        // Update current playback position
        if let Some(player) = &self.audio_player {
            self.current_position = player.position();
        }
        
        // Handle dropped files
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                if let Some(file) = i.raw.dropped_files.first() {
                    if let Some(path) = &file.path {
                        self.handle_dropped_file(path.clone());
                    }
                }
            }
        });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("SRT Segment Merger");
            });
            ui.separator();
            
            ui.horizontal(|ui| {
                // Left panel: Drop area and player
                ui.vertical(|ui| {
                    ui.set_width(700.0);
                    
                    // Drop area
                    egui::Frame::default()
                        .fill(egui::Color32::from_rgb(40, 40, 50))
                        .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 100, 120)))
                        .inner_margin(20.0)
                        .show(ui, |ui| {
                            ui.set_min_height(100.0);
                            ui.vertical_centered(|ui| {
                                if let Some(path) = &self.video_path {
                                    ui.label(format!("📹 {}", path.file_name().unwrap().to_string_lossy()));
                                } else {
                                    ui.label("📂 Drag video file here");
                                }
                            });
                        });
                    
                    ui.add_space(10.0);
                    
                    // Audio player
                    if self.state != AppState::Idle {
                        egui::Frame::default()
                            .fill(egui::Color32::from_rgb(30, 30, 40))
                            .inner_margin(15.0)
                            .show(ui, |ui| {
                                ui.label("Audio Player");
                                ui.separator();
                                
                                // Time display
                                ui.horizontal(|ui| {
                                    ui.label(Self::format_time(self.current_position));
                                    ui.label("/");
                                    ui.label(Self::format_time(self.total_duration));
                                });
                                
                                ui.add_space(5.0);
                                
                                // Playback progress bar (full width)
                                let mut position = self.current_position;
                                // 使用进度条宽度等于左侧面板宽度减去边距
                                ui.spacing_mut().slider_width = 640.0;
                                if ui.add(egui::Slider::new(&mut position, 0.0..=self.total_duration)
                                    .show_value(false)).changed() {
                                    self.current_position = position;
                                    if let Some(player) = &mut self.audio_player {
                                        player.seek(position);
                                    }
                                }
                                ui.add_space(5.0);
                                
                                ui.horizontal(|ui| {
                                    // Play/Pause button
                                    if self.is_playing {
                                        if ui.button("Pause").clicked() {
                                            if let Some(player) = &mut self.audio_player {
                                                player.pause();
                                                self.is_playing = false;
                                            }
                                        }
                                    } else {
                                        if ui.button("Play").clicked() {
                                            if let Some(player) = &mut self.audio_player {
                                                player.play();
                                                self.is_playing = true;
                                            }
                                        }
                                    }
                                })
                            });
                    }
                    
                    ui.add_space(10.0);
                    
                    // Load Complete SRT section
                    ui.separator();
                    ui.label("Complete SRT File");
                    
                    ui.horizontal(|ui| {
                        if ui.button("Load Complete SRT").clicked() {
                            self.load_srt_file();
                        }
                        
                        ui.add(egui::TextEdit::singleline(&mut self.complete_srt_path)
                            .hint_text("Or enter complete SRT path...")
                            .desired_width(400.0));
                    });
                    
                    if self.complete_srt_loaded {
                        ui.label("Complete SRT loaded");
                    }
                    
                    ui.add_space(5.0);
                    
                    // Load Segment SRT section
                    ui.label("Segment SRT File (Optional)");
                    
                    ui.horizontal(|ui| {
                        if ui.button("Load Segment SRT").clicked() {
                            self.load_segment_srt_file();
                        }
                        
                        ui.add(egui::TextEdit::singleline(&mut self.segment_srt_path)
                            .hint_text("Or enter segment SRT path...")
                            .desired_width(400.0));
                    });
                    
                    if self.segment_srt_loaded {
                        ui.label("Segment SRT loaded");
                    }
                    
                    ui.add_space(10.0);
                    
                    // Manual cut section
                    if self.state != AppState::Idle && self.state != AppState::Processing {
                        ui.separator();
                        ui.label("Manual Cut Segment");
                        
                        // Start time
                        ui.horizontal(|ui| {
                            ui.label("Start:");
                            ui.add(egui::TextEdit::singleline(&mut self.manual_start_hour)
                                .desired_width(30.0)
                                .hint_text("00"));
                            ui.label("h");
                            ui.add(egui::TextEdit::singleline(&mut self.manual_start_minute)
                                .desired_width(30.0)
                                .hint_text("00"));
                            ui.label("m");
                            ui.add(egui::TextEdit::singleline(&mut self.manual_start_second)
                                .desired_width(30.0)
                                .hint_text("00"));
                            ui.label("s");
                            ui.add(egui::TextEdit::singleline(&mut self.manual_start_millisecond)
                                .desired_width(40.0)
                                .hint_text("000"));
                            ui.label("ms");
                        });
                        
                        // End time
                        ui.horizontal(|ui| {
                            ui.label("End:  ");
                            ui.add(egui::TextEdit::singleline(&mut self.manual_end_hour)
                                .desired_width(30.0)
                                .hint_text("00"));
                            ui.label("h");
                            ui.add(egui::TextEdit::singleline(&mut self.manual_end_minute)
                                .desired_width(30.0)
                                .hint_text("00"));
                            ui.label("m");
                            ui.add(egui::TextEdit::singleline(&mut self.manual_end_second)
                                .desired_width(30.0)
                                .hint_text("00"));
                            ui.label("s");
                            ui.add(egui::TextEdit::singleline(&mut self.manual_end_millisecond)
                                .desired_width(40.0)
                                .hint_text("000"));
                            ui.label("ms");
                        });
                        
                        ui.label("Empty fields default to 0");
                        
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            if ui.button("Cut Segment").clicked() {
                                self.cut_manual_segment();
                            }
                            
                            if self.manual_segment.is_some() {
                                if ui.button("Recognize Segment").clicked() {
                                    self.recognize_manual_segment();
                                }
                            }
                            
                            // Show Merge button if either complete SRT is loaded and (segment is recognized OR segment SRT is loaded)
                            let can_merge = self.complete_srt_loaded && 
                                (self.manual_segment.is_some() || self.segment_srt_loaded);
                            
                            if can_merge {
                                if ui.button("Merge to SRT").clicked() {
                                    self.merge_segment_subtitle();
                                }
                            }
                        });
                    }
                    
                    ui.add_space(10.0);
                    
                    // Status message
                    ui.label(&self.status_message);
                });
                
                ui.separator();
                
                // Right panel: Settings
                ui.vertical(|ui| {
                    ui.set_width(400.0);
                    
                    ui.heading("Settings");
                    ui.separator();
                    
                    // Whisper model selection
                    ui.label("Whisper Model:");
                    egui::ComboBox::from_label("")
                        .selected_text(self.whisper_model.as_str())
                        .show_ui(ui, |ui| {
                            for model in WhisperModel::all() {
                                ui.selectable_value(&mut self.whisper_model, model, model.as_str());
                            }
                        });
                    
                    ui.add_space(10.0);
                    
                    // Language selection
                    ui.label("Language:");
                    egui::ComboBox::from_label(" ")
                        .selected_text(self.whisper_language.as_str())
                        .show_ui(ui, |ui| {
                            for lang in WhisperLanguage::all() {
                                ui.selectable_value(&mut self.whisper_language, lang.clone(), lang.as_str());
                            }
                        });
                    
                    // Custom language input (only show when Custom is selected)
                    if self.whisper_language == WhisperLanguage::Custom {
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            ui.label("Language code:");
                            ui.text_edit_singleline(&mut self.custom_language_code);
                        });
                        ui.label("Examples: ko (Korean), ar (Arabic), hi (Hindi), pt (Portuguese)");
                    }
                    
                    ui.add_space(20.0);
                    ui.separator();
                    
                    // Recognition section
                    ui.label("Recognition");
                    ui.add_space(5.0);
                    
                    if !self.audio_segments.is_empty() {
                        ui.label(format!("Audio segments: {}", self.audio_segments.len()));
                        ui.add_space(10.0);
                        
                        if self.state != AppState::Processing {
                            if ui.button("Start Recognition").clicked() {
                                self.start_recognition();
                            }
                        } else {
                            ui.label("Recognizing...");
                            ui.label(&self.processing_status);
                            ui.add_space(5.0);
                            ui.add(egui::ProgressBar::new(self.processing_progress).show_percentage());
                            ui.add_space(5.0);
                            if ui.button("Stop Recognition & Kill Processes").clicked() {
                                self.stop_recognition();
                            }
                        }
                        
                        ui.add_space(10.0);
                        
                        // Recognition results / Whisper log output
                        if !self.recognition_results.is_empty() {
                            ui.separator();
                            ui.label("Whisper Output Log:");
                            ui.add_space(5.0);
                            
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .max_height(250.0)
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    egui::Frame::default()
                                        .fill(egui::Color32::from_rgb(20, 20, 25))
                                        .inner_margin(10.0)
                                        .show(ui, |ui| {
                                            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
                                            for result in &self.recognition_results {
                                                ui.label(result);
                                            }
                                        });
                                });
                        }
                    } else {
                        ui.label("No audio segments");
                    }
                    
                    ui.add_space(10.0);
                });
            });
        });
        
        // Continuously refresh UI to update playback position
        ctx.request_repaint();
    }
}

