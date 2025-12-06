# Whisper 语音识别工具

一个基于 Rust 和 OpenAI Whisper 的图形界面语音识别工具，专门用于处理长视频的语音识别任务。

## 功能特点

- 🎬 **视频音频提取**: 自动检测并提取视频中的音频轨道
- ✂️ **智能音频切割**: 可视化标记切割点，将长音频分段处理
- 🎵 **音频播放控制**: 内置播放器，支持播放/暂停、进度条拖动
- 🎤 **多模型支持**: 支持 Whisper 的 tiny、base、small、medium、large 模型
- 🌍 **多语言识别**: 支持日语、英语、中文、法语、德语、西班牙语等，或自动识别
- 📝 **自动字幕合并**: 智能合并多段字幕，保持时间戳精确对齐

## 系统要求

- Rust 1.70+
- FFmpeg (用于音视频处理)
- OpenAI Whisper (用于语音识别)

## 安装依赖

### macOS
```bash
brew install ffmpeg
pip install openai-whisper
```

### Linux
```bash
sudo apt install ffmpeg
pip install openai-whisper
```

## 编译运行

```bash
# 编译
cargo build --release

# 运行
cargo run --release
```

## 使用方法

1. **加载视频**: 拖拽视频文件到左侧窗口
2. **音频提取**: 程序会自动调用 FFmpeg 提取音频
3. **标记切割点**: 
   - 使用播放控制按钮播放音频
   - 在需要切割的位置点击"标记切割点"按钮
   - 重复此步骤标记所有切割点
4. **执行切割**: 点击"执行切割"按钮，将音频分段
5. **设置参数**: 
   - 选择 Whisper 模型 (推荐 base 或 medium)
   - 选择识别语言（或选择"自动识别"）
6. **开始识别**: 点击"开始识别"按钮
7. **等待完成**: 识别完成后，字幕文件会保存在视频同目录下

## 项目结构

```
src/
├── main.rs           # 主程序和 GUI 界面
├── audio_player.rs   # 音频播放器实现
├── ffmpeg.rs         # FFmpeg 调用封装
├── whisper.rs        # Whisper 调用封装
└── srt_merger.rs     # SRT 字幕合并算法
```

## 技术栈

- **GUI 框架**: egui (immediate mode GUI)
- **音频播放**: rodio
- **异步运行时**: tokio
- **命令行调用**: FFmpeg, Whisper

## 字幕合并算法

程序使用精确的时间戳计算算法：

1. 解析每段音频生成的 SRT 文件
2. 根据切割点计算每段的时间偏移量
3. 调整每条字幕的开始和结束时间
4. 按时间顺序合并所有字幕
5. 重新编号并输出最终的 SRT 文件

这确保了合并后的字幕与原始视频完美同步。

## 注意事项

- 确保系统已安装 FFmpeg 和 Whisper
- 长视频建议使用切割功能，提高识别准确度
- 不同模型的识别速度和准确度不同：
  - tiny: 最快，准确度较低
  - base: 平衡速度和准确度
  - medium: 较慢，准确度高
  - large: 最慢，准确度最高

## 许可证

MIT License

