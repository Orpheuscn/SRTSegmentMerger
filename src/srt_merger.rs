use std::path::Path;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use anyhow::{Result, anyhow};

#[derive(Debug, Clone)]
pub struct SubtitleEntry {
    pub index: usize,
    pub start_time: f64,  // in seconds
    pub end_time: f64,    // in seconds
    pub text: Vec<String>,
}

/// Parse SRT time string to seconds
fn parse_srt_time(time_str: &str) -> Result<f64> {
    // Format: HH:MM:SS,mmm
    let time_str = time_str.trim();
    
    if time_str.is_empty() {
        return Err(anyhow!("Empty time string"));
    }
    
    let parts: Vec<&str> = time_str.split(',').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid time format: {} (expected HH:MM:SS,mmm)", time_str));
    }
    
    let time_parts: Vec<&str> = parts[0].split(':').collect();
    if time_parts.len() != 3 {
        return Err(anyhow!("Invalid time format: {} (expected HH:MM:SS,mmm)", time_str));
    }
    
    let hours: f64 = time_parts[0].trim().parse()
        .map_err(|_| anyhow!("Invalid hour value: {}", time_parts[0]))?;
    let minutes: f64 = time_parts[1].trim().parse()
        .map_err(|_| anyhow!("Invalid minute value: {}", time_parts[1]))?;
    let seconds: f64 = time_parts[2].trim().parse()
        .map_err(|_| anyhow!("Invalid second value: {}", time_parts[2]))?;
    let milliseconds: f64 = parts[1].trim().parse()
        .map_err(|_| anyhow!("Invalid millisecond value: {}", parts[1]))?;
    
    Ok(hours * 3600.0 + minutes * 60.0 + seconds + milliseconds / 1000.0)
}

/// Convert seconds to SRT time format
fn format_srt_time(seconds: f64) -> String {
    let hours = (seconds / 3600.0).floor() as u32;
    let minutes = ((seconds % 3600.0) / 60.0).floor() as u32;
    let secs = (seconds % 60.0).floor() as u32;
    let millis = ((seconds % 1.0) * 1000.0).floor() as u32;
    
    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, secs, millis)
}

/// Parse a single SRT file
pub fn parse_srt_file(path: &Path) -> Result<Vec<SubtitleEntry>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    
    let mut lines = reader.lines();
    let mut current_entry: Option<SubtitleEntry> = None;
    
    while let Some(line) = lines.next() {
        let line = line?;
        let line = line.trim();
        
        if line.is_empty() {
            if let Some(entry) = current_entry.take() {
                if entry.start_time >= 0.0 && entry.end_time >= 0.0 && !entry.text.is_empty() {
                    entries.push(entry);
                }
            }
            continue;
        }
        
        // Try to parse index
        if let Ok(index) = line.parse::<usize>() {
            current_entry = Some(SubtitleEntry {
                index,
                start_time: -1.0,
                end_time: -1.0,
                text: Vec::new(),
            });
            continue;
        }
        
        // Try to parse time line
        if line.contains("-->") {
            let time_parts: Vec<&str> = line.split("-->").collect();
            if time_parts.len() == 2 {
                if let Some(ref mut entry) = current_entry {
                    let start = time_parts[0].trim();
                    let end = time_parts[1].trim();
                    if let (Ok(start_time), Ok(end_time)) = (parse_srt_time(start), parse_srt_time(end)) {
                        entry.start_time = start_time;
                        entry.end_time = end_time;
                    }
                }
            }
            continue;
        }
        
        // Subtitle text
        if let Some(ref mut entry) = current_entry {
            if entry.start_time >= 0.0 {
                entry.text.push(line.to_string());
            }
        }
    }
    
    // Add last entry
    if let Some(entry) = current_entry {
        if entry.start_time >= 0.0 && entry.end_time >= 0.0 && !entry.text.is_empty() {
            entries.push(entry);
        }
    }
    
    Ok(entries)
}

/// Adjust segment subtitle times by adding offset
pub fn adjust_segment_times(segment_subs: &[SubtitleEntry], offset: f64) -> Vec<SubtitleEntry> {
    segment_subs.iter().map(|sub| {
        SubtitleEntry {
            index: sub.index,
            start_time: sub.start_time + offset,
            end_time: sub.end_time + offset,
            text: sub.text.clone(),
        }
    }).collect()
}

/// Merge segment subtitle into complete subtitle
pub fn merge_subtitles(
    complete_subs: Vec<SubtitleEntry>,
    segment_subs: Vec<SubtitleEntry>,
) -> Vec<SubtitleEntry> {
    let mut all_subs = complete_subs;
    all_subs.extend(segment_subs);
    
    // Sort by start time
    all_subs.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
    
    // Renumber
    for (i, sub) in all_subs.iter_mut().enumerate() {
        sub.index = i + 1;
    }
    
    all_subs
}

/// Write SRT file
pub fn write_srt_file(path: &Path, subtitles: &[SubtitleEntry]) -> Result<()> {
    let mut file = File::create(path)?;
    
    for (i, entry) in subtitles.iter().enumerate() {
        writeln!(file, "{}", entry.index)?;
        writeln!(file, "{} --> {}", format_srt_time(entry.start_time), format_srt_time(entry.end_time))?;
        for line in &entry.text {
            writeln!(file, "{}", line)?;
        }
        if i < subtitles.len() - 1 {
            writeln!(file)?;
        }
    }
    
    Ok(())
}

/// Insert segment subtitle into complete subtitle at the specified time offset
pub fn insert_segment_subtitle(
    complete_srt_path: &Path,
    segment_srt_path: &Path,
    segment_start_time: f64,
    output_path: &Path,
) -> Result<()> {
    // Parse complete subtitle
    let complete_subs = parse_srt_file(complete_srt_path)?;
    
    // Parse segment subtitle
    let segment_subs = parse_srt_file(segment_srt_path)?;
    
    // Adjust segment times
    let adjusted_segment = adjust_segment_times(&segment_subs, segment_start_time);
    
    // Merge
    let merged = merge_subtitles(complete_subs, adjusted_segment);
    
    // Write output
    write_srt_file(output_path, &merged)?;
    
    Ok(())
}
