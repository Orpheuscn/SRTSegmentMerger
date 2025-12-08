# Whisper Speech Recognition

基于 OpenAI Whisper 的语音识别工具，专为长视频设计。通过手动标记音频切割点，避免 Whisper 在处理长音频时出现的识别错误和空缺问题。

## 背景

Whisper 在处理长视频时存在以下问题：
- 识别准确率下降
- 出现大量空缺
- 时间轴错位

常见的解决方案是自动切割（如每5分钟一段），但这会截断句子，导致识别失败。本工具通过可视化界面手动标记切割点，在静音或语句间隙处切割，确保 Whisper 发挥稳定。

## 技术栈

- **Rust** - 主程序语言
- **egui** - GUI 框架（immediate mode）
- **rodio** - 音频播放
- **FFmpeg** - 音视频处理
- **OpenAI Whisper** - 语音识别引擎
- **Tokio** - 异步运行时

## 系统要求

- Rust 1.70+
- FFmpeg
- OpenAI Whisper

## 安装依赖

### 安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 安装 FFmpeg 和 Whisper

**macOS:**
```bash
brew install ffmpeg
pip3 install openai-whisper
```

**Linux:**
```bash
sudo apt install ffmpeg
pip3 install openai-whisper
```

## 编译

```bash
cd path/to/WhisperSpeechRecognition
cargo build --release
```

编译后的可执行文件位于 `target/release/whisper-gui`。

## 运行

```bash
cargo run --release
```

或直接运行编译后的可执行文件：
```bash
./target/release/whisper-gui
```

## 使用指南

### 1. 加载视频

点击"选择视频文件"按钮或拖拽视频到窗口，程序自动提取音频轨道。

### 2. 标记切割点

- 使用播放控制按钮（播放/暂停）
- 拖动进度条定位到需要切割的位置
- 在语句间隙或静音处点击"标记切割点"
- 重复标记所有切割点

### 3. 执行切割

点击"执行切割"，将音频按标记点分段。

### 4. 配置识别参数

**选择 Whisper 模型：**
- `tiny` - 最快，准确率较低
- `base` - 平衡速度和准确率（推荐）
- `small` - 较慢，准确率较高
- `medium` - 慢，准确率高
- `large` - 最慢，准确率最高

**选择语言：**
- 中文、日语、英语、法语、德语、西班牙语等
- 或选择"自动识别"

### 5. 开始识别

点击"开始识别"，等待处理完成。字幕文件（.srt）会保存在视频同目录下。

## 界面功能

- **视频区域** - 显示视频信息和音频提取进度
- **播放控制** - 播放/暂停按钮和进度条
- **切割点管理** - 标记、查看和删除切割点
- **参数设置** - 模型选择、语言选择
- **识别控制** - 启动识别任务和查看进度

## 字幕合并算法

程序自动合并多段音频的识别结果：
1. 解析每段生成的 SRT 文件
2. 根据切割点计算时间偏移量
3. 调整字幕时间戳
4. 按时间顺序合并并重新编号
5. 输出最终 SRT 文件

## 注意事项

- 建议在句子间隙或静音处标记切割点
- 避免在单词或句子中间切割
- 模型越大，识别越准确，但耗时更长
- 确保系统已安装 FFmpeg 和 Whisper

## 许可证

MIT License
