use crate::cli::Cli;
use crate::progress::ProgressManager;
use crate::video_info::VideoInfo;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::fs;
use std::time::Instant;

pub fn encode_video(cli: &Cli, input_path: &Path, progress: &mut ProgressManager) -> Result<()> {
    let video_info = VideoInfo::from_file(input_path)?;
    
    // Check if we should skip VP9 encoding
    if cli.skip_vp9 && video_info.is_vp9() {
        println!("\n════════════════════════════════════════════════════════════════════");
        println!("⏭️  SKIPPING: {} (already VP9)", input_path.file_name().unwrap_or_default().to_string_lossy());
        println!("════════════════════════════════════════════════════════════════════");
        println!("  Video codec: {} - No encoding needed", video_info.video_codec);
        println!("  Use --skip-vp9 flag to process anyway");
        println!("════════════════════════════════════════════════════════════════════\n");
        return Ok(());
    }
    
    // Determine output path
    let output_path = determine_output_path(cli, input_path)?;
    
    // Prepare output directory
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Get source file size
    let source_size = fs::metadata(input_path)?.len();
    let source_size_mb = source_size as f64 / 1_048_576.0;
    let source_bitrate_estimate = (source_size as f64 * 8.0) / video_info.duration / 1_000_000.0;
    
    let pixels = video_info.width as u64 * video_info.height as u64;
    
    println!("\n════════════════════════════════════════════════════════════════════");
    println!("📹 Source: {}", input_path.file_name().unwrap_or_default().to_string_lossy());
    println!("════════════════════════════════════════════════════════════════════");
    println!("  Resolution: {}x{} ({:.1} Mpx)", 
             video_info.width, video_info.height, pixels as f64 / 1_000_000.0);
    println!("  Duration: {:.2} sec ({:.2} min)", video_info.duration, video_info.duration / 60.0);
    println!("  Size: {:.2} MB", source_size_mb);
    println!("  Estimated source bitrate: {:.2} Mbps", source_bitrate_estimate);
    println!("  Video codec: {}", video_info.video_codec);
    if let Some(fps) = video_info.framerate {
        println!("  Framerate: {:.2} fps", fps);
    }
    println!("  Audio tracks: {}", video_info.audio_tracks.len());
    for track in &video_info.audio_tracks {
        let source_br = match track.channels {
            0 | 1 | 2 => 128,
            3 | 4 | 5 => 256,
            6 | 7 => 384,
            _ => 512,
        };
        println!("      - Track {} ({}): {} channels, codec {} (source ~{} kbps)", 
                 track.stream_order, track.language, track.channels, track.codec, source_br);
    }
    println!("  Subtitle tracks: {}", video_info.subtitle_tracks.len());
    for track in &video_info.subtitle_tracks {
        println!("      - Track {} ({}): {}", track.stream_order, track.language, track.codec);
    }
    
    println!("\n🎯 Output: {}", output_path.file_name().unwrap_or_default().to_string_lossy());
    
    // Build ffmpeg command
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-i").arg(input_path);
    
    // Map streams using ffmpeg's stream specifiers (more reliable, ignores cover art)
    // v:0 = first video stream, a = all audio, s = all subtitles
    cmd.arg("-map").arg("0:v:0");
    cmd.arg("-map").arg("0:a");
    
    if !video_info.subtitle_tracks.is_empty() {
        cmd.arg("-map").arg("0:s");
    }
    
    // Video encoding
    let video_bitrate = determine_video_bitrate(cli, &video_info)?;
    let crf_value = determine_crf(cli, &video_info)?;
    
    cmd.arg("-c:v").arg("libvpx-vp9");
    cmd.arg("-row-mt").arg("1");
    
    let target_video_bitrate: Option<u32>;
    if let Some(bitrate) = video_bitrate {
        println!("  Video target bitrate: {} bps ({:.2} Mbps)", bitrate, bitrate as f64 / 1_000_000.0);
        cmd.arg("-b:v").arg(bitrate.to_string());
        
        // Add minimum CRF to force bitrate compliance (fix for ffmpeg VP9 bitrate issue)
        println!("  Force minimum quality: CRF {}", cli.bv_min_crf);
        cmd.arg("-crf").arg(cli.bv_min_crf.to_string());
        
        target_video_bitrate = Some(bitrate);
    } else if let Some(crf) = crf_value {
        println!("  Video CRF: {} (quality-based, variable bitrate)", crf);
        cmd.arg("-crf").arg(crf.to_string());
        target_video_bitrate = None;
    } else {
        let crf = calculate_auto_crf(video_info.width, video_info.height);
        println!("  Video CRF: {} (auto, based on {}x{})", crf, video_info.width, video_info.height);
        cmd.arg("-crf").arg(crf.to_string());
        target_video_bitrate = None;
    }
    
    // Framerate adjustment if requested
    if let Some(fps) = cli.fps {
        println!("  Framerate: changing from {} to {} fps", 
                 video_info.framerate.map_or("?".to_string(), |f| format!("{:.2}", f)), 
                 fps);
        cmd.arg("-r").arg(fps.to_string());
        cmd.arg("-g").arg(((fps * 10.0) as u32).to_string()); // GOP size: 10 seconds
    } else if let Some(original_fps) = video_info.framerate {
        cmd.arg("-r").arg(original_fps.to_string());
        cmd.arg("-g").arg(((original_fps * 10.0) as u32).to_string());
    }
    
    // Speed settings
    let (deadline, speed) = cli.speed.to_ffmpeg_args();
    cmd.arg("-deadline").arg(&deadline);
    cmd.arg("-speed").arg(&speed);
    println!("  Speed: {:?} (deadline: {}, speed: {})", cli.speed, deadline, speed);
    
    // Audio encoding
    cmd.arg("-c:a").arg("libopus");
    
    let mut target_audio_bitrates = Vec::new();
    for (idx, track) in video_info.audio_tracks.iter().enumerate() {
        let bitrate = determine_audio_bitrate(cli, track.channels)?;
        let br = if let Some(b) = bitrate {
            println!("  Audio track {} ({}): {} channels -> {} bps ({:.2} kbps)", 
                     track.stream_order, track.language, track.channels, b, b as f64 / 1000.0);
            b
        } else {
            let auto_br = calculate_auto_audio_bitrate(track.channels);
            println!("  Audio track {} ({}): {} channels -> {} bps (auto, {:.2} kbps)", 
                     track.stream_order, track.language, track.channels, auto_br, auto_br as f64 / 1000.0);
            auto_br
        };
        
        cmd.arg(format!("-b:a:{}", idx)).arg(format!("{}k", br / 1000));
        cmd.arg(format!("-ac:a:{}", idx)).arg(track.channels.to_string());
        
        target_audio_bitrates.push(br);
    }
    
    let target_audio_total_bitrate: u32 = target_audio_bitrates.iter().sum();
    if !target_audio_bitrates.is_empty() {
        println!("  Total audio target: {:.2} Mbps", target_audio_total_bitrate as f64 / 1_000_000.0);
    }
    
    // Subtitles: copy without re-encoding
    if !video_info.subtitle_tracks.is_empty() {
        cmd.arg("-c:s").arg("copy");
    }
    
    // Copy useful metadata but remove statistics
    cmd.arg("-map_metadata").arg("0:g");
    cmd.arg("-map_metadata:s:v").arg("0:s:v");
    cmd.arg("-map_metadata:s:a").arg("0:s:a");
    if !video_info.subtitle_tracks.is_empty() {
        cmd.arg("-map_metadata:s:s").arg("0:s:s");
    }
    
    // Remove statistics metadata for video stream
    cmd.arg("-metadata:s:v").arg("BPS=");
    cmd.arg("-metadata:s:v").arg("DURATION=");
    cmd.arg("-metadata:s:v").arg("NUMBER_OF_FRAMES=");
    cmd.arg("-metadata:s:v").arg("NUMBER_OF_BYTES=");
    cmd.arg("-metadata:s:v").arg("_STATISTICS_WRITING_APP=");
    cmd.arg("-metadata:s:v").arg("_STATISTICS_WRITING_DATE_UTC=");
    cmd.arg("-metadata:s:v").arg("_STATISTICS_TAGS=");
    cmd.arg("-metadata:s:v").arg("ENCODER=");
    
    // Remove statistics metadata for each audio track
    for idx in 0..video_info.audio_tracks.len() {
        cmd.arg(format!("-metadata:s:a:{}", idx)).arg("BPS=");
        cmd.arg(format!("-metadata:s:a:{}", idx)).arg("DURATION=");
        cmd.arg(format!("-metadata:s:a:{}", idx)).arg("NUMBER_OF_FRAMES=");
        cmd.arg(format!("-metadata:s:a:{}", idx)).arg("NUMBER_OF_BYTES=");
        cmd.arg(format!("-metadata:s:a:{}", idx)).arg("_STATISTICS_WRITING_APP=");
        cmd.arg(format!("-metadata:s:a:{}", idx)).arg("_STATISTICS_WRITING_DATE_UTC=");
        cmd.arg(format!("-metadata:s:a:{}", idx)).arg("_STATISTICS_TAGS=");
    }
    
    // Remove statistics metadata for each subtitle track
    for idx in 0..video_info.subtitle_tracks.len() {
        cmd.arg(format!("-metadata:s:s:{}", idx)).arg("BPS=");
        cmd.arg(format!("-metadata:s:s:{}", idx)).arg("DURATION=");
        cmd.arg(format!("-metadata:s:s:{}", idx)).arg("NUMBER_OF_FRAMES=");
        cmd.arg(format!("-metadata:s:s:{}", idx)).arg("NUMBER_OF_BYTES=");
        cmd.arg(format!("-metadata:s:s:{}", idx)).arg("_STATISTICS_WRITING_APP=");
        cmd.arg(format!("-metadata:s:s:{}", idx)).arg("_STATISTICS_WRITING_DATE_UTC=");
        cmd.arg(format!("-metadata:s:s:{}", idx)).arg("_STATISTICS_TAGS=");
    }
    
    // Output
    cmd.arg("-y").arg(&output_path);
    
    // Progress monitoring
    cmd.arg("-progress").arg("pipe:1");
    cmd.arg("-nostats");
    
    println!("\n⏳ Encoding in progress...\n");
    
    // Execute with progress monitoring
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    // Monitor progress from stdout
    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);
    
    let total_duration = video_info.duration;
    let start_time = Instant::now();
    let output_path_clone = output_path.clone();
    
    let mut last_progress_update = Instant::now();
    let mut _last_time_ms = 0u64;
    let mut last_encoded_seconds = 0.0;
    let mut first_update = true;
    
    for line in reader.lines() {
        let line = line?;
        if let Some(time_ms) = parse_out_time(&line) {
            _last_time_ms = time_ms;
            let elapsed = start_time.elapsed().as_secs_f64();
            let encoded_seconds = time_ms as f64 / 1_000_000.0;
            last_encoded_seconds = encoded_seconds;
            let progress_percent = (encoded_seconds / total_duration).min(1.0);
            
            progress.update_progress(progress_percent);
            
            if last_progress_update.elapsed().as_secs() >= 1 || first_update {
                last_progress_update = Instant::now();
                first_update = false;
                
                let encoding_speed = if elapsed > 0.0 {
                    encoded_seconds / elapsed
                } else {
                    0.0
                };
                
                let remaining_seconds = if encoding_speed > 0.0 && progress_percent < 1.0 {
                    (total_duration - encoded_seconds) / encoding_speed
                } else {
                    0.0
                };
                
                let current_bitrate_mbps = if encoded_seconds > 10.0 {
                    if let Ok(metadata) = fs::metadata(&output_path_clone) {
                        let file_size_bits = metadata.len() as f64 * 8.0;
                        Some((file_size_bits / encoded_seconds) / 1_000_000.0)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                let estimated_final_size_mb = if progress_percent > 0.01 {
                    if let Ok(metadata) = fs::metadata(&output_path_clone) {
                        let current_size_mb = metadata.len() as f64 / 1_048_576.0;
                        Some(current_size_mb / progress_percent)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                let progress_pct = progress_percent * 100.0;
                let encoded_min = encoded_seconds / 60.0;
                let eta_min = remaining_seconds / 60.0;
                let _elapsed_min = elapsed / 60.0;
                
                let bar_width = 30;
                let filled = (progress_pct / 100.0 * bar_width as f64) as usize;
                let bar = format!("[{}{}]", 
                    "=".repeat(filled),
                    " ".repeat(bar_width - filled));
                
                let mut info = format!("{:5.1}% {} {:4.1}/{:4.1}min {:4.2}x",
                    progress_pct, bar, encoded_min, total_duration / 60.0, encoding_speed);
                
                if eta_min.is_finite() && eta_min > 0.0 {
                    info.push_str(&format!(" ETA:{:5.1}min", eta_min));
                }
                
                if let Some(br) = current_bitrate_mbps {
                    info.push_str(&format!(" {:.1}Mbps", br));
                }
                
                if let Some(est) = estimated_final_size_mb {
                    info.push_str(&format!(" ->{:.0}MB", est));
                }
                
                print!("\r\x1b[K{}", info);
                use std::io::Write;
                std::io::stdout().flush().unwrap();
            }
        }
    }
    
    println!("\n");
    
    let status = child.wait()?;
    
    if !status.success() {
        return Err(anyhow!("ffmpeg exited with error code: {:?}", status.code()));
    }
    
    let total_elapsed = start_time.elapsed().as_secs_f64();
    let final_encoding_speed = if total_elapsed > 0.0 {
        last_encoded_seconds / total_elapsed
    } else {
        0.0
    };
    let final_size = fs::metadata(&output_path)?.len();
    let final_bitrate = (final_size as f64 * 8.0) / last_encoded_seconds;
    let final_bitrate_mbps = final_bitrate / 1_000_000.0;
    let gain = source_size as f64 / final_size as f64;
    
    print!("\r\x1b[K");
    println!("\n✅ Encoding completed!");
    println!("════════════════════════════════════════════════════════════════════");
    println!("  Time: {:.2} min ({:.1} sec)", total_elapsed / 60.0, total_elapsed);
    println!("  Speed: {:.2}x (real-time)", final_encoding_speed);
    println!("  Final size: {:.2} MB (gain: {:.1}%)", 
             final_size as f64 / 1_048_576.0, (1.0 - 1.0/gain) * 100.0);
    println!("  Average bitrate: {:.2} Mbps", final_bitrate_mbps);
    
    if let Some(target_br) = target_video_bitrate {
        let target_br_mbps = target_br as f64 / 1_000_000.0;
        let diff = (final_bitrate_mbps - target_br_mbps).abs();
        let diff_percent = if target_br_mbps > 0.0 { (diff / target_br_mbps) * 100.0 } else { 0.0 };
        println!("  Target: {:.2} Mbps (diff: {:.1}%)", target_br_mbps, diff_percent);
    }
    println!("════════════════════════════════════════════════════════════════════");
    
    Ok(())
}

pub fn determine_output_path(cli: &Cli, input_path: &Path) -> Result<PathBuf> {
    if let Some(output) = &cli.output {
        if output.exists() {
            return make_unique_path(output);
        }
        return Ok(output.clone());
    }
    
    let base_dir = if let Some(output_dir) = &cli.output_dir {
        output_dir.clone()
    } else if let Some(input_dir) = &cli.input_dir {
        input_dir.clone()
    } else if let Some(parent) = input_path.parent() {
        parent.to_path_buf()
    } else {
        PathBuf::from(".")
    };
    
    let relative_path = if cli.recursive && cli.input_dir.is_some() {
        if let Some(input_dir) = &cli.input_dir {
            input_path.strip_prefix(input_dir).unwrap_or(input_path)
        } else {
            input_path
        }
    } else {
        input_path
    };
    
    let output_name = relative_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
        + ".mkv";
    
    let output_path = base_dir.join(relative_path.parent().unwrap_or(Path::new(""))).join(output_name);
    
    if output_path.exists() {
        make_unique_path(&output_path)
    } else {
        Ok(output_path)
    }
}

fn make_unique_path(path: &Path) -> Result<PathBuf> {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let extension = path.extension().unwrap_or_default().to_string_lossy();
    let parent = path.parent().unwrap_or(Path::new(""));
    
    for i in 1..10000 {
        let new_name = format!("{}_{:04}.{}", stem, i, extension);
        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return Ok(new_path);
        }
    }
    
    Err(anyhow!("Could not find unique filename for {}", path.display()))
}

pub fn post_process(cli: &Cli, input_path: &Path, output_path: &Path) -> Result<()> {
    if cli.delete || cli.rename {
        let original_name = input_path.file_name().unwrap_or_default();
        let output_name = output_path.file_name().unwrap_or_default();
        
        if cli.delete {
            println!("\n  Post-processing:");
            println!("     Deleting source: {}", input_path.display());
            std::fs::remove_file(input_path)?;
            println!("     Renaming output: {} -> {}", output_name.to_string_lossy(), original_name.to_string_lossy());
            let final_path = output_path.parent().unwrap().join(original_name);
            std::fs::rename(output_path, &final_path)?;
            println!("     Done");
        } else if cli.rename {
            println!("\n  Post-processing:");
            let old_name = format!("_old_{}", original_name.to_string_lossy());
            let old_path = input_path.parent().unwrap().join(&old_name);
            println!("     Renaming source: {} -> {}", input_path.display(), old_path.display());
            std::fs::rename(input_path, &old_path)?;
            println!("     Renaming output: {} -> {}", output_name.to_string_lossy(), original_name.to_string_lossy());
            let final_path = output_path.parent().unwrap().join(original_name);
            std::fs::rename(output_path, &final_path)?;
            println!("     Done (original renamed to: {})", old_name);
        }
    }
    Ok(())
}

fn determine_video_bitrate(cli: &Cli, info: &VideoInfo) -> Result<Option<u32>> {
    if let Some(bitrate_str) = &cli.video_bitrate {
        if bitrate_str == "auto" {
            return Ok(Some(calculate_auto_bitrate(info.width, info.height)));
        }
        return Ok(Some(parse_bitrate(bitrate_str)?));
    }
    Ok(None)
}

fn determine_crf(cli: &Cli, info: &VideoInfo) -> Result<Option<u32>> {
    if let Some(crf_str) = &cli.crf {
        if crf_str == "auto" {
            return Ok(Some(calculate_auto_crf(info.width, info.height)));
        }
        let crf = crf_str.parse::<u32>()?;
        if crf < 8 || crf > 48 {
            return Err(anyhow!("CRF must be between 8 and 48"));
        }
        return Ok(Some(crf));
    }
    Ok(None)
}

fn determine_audio_bitrate(cli: &Cli, channels: u32) -> Result<Option<u32>> {
    if let Some(bitrate_str) = &cli.audio_bitrate {
        if bitrate_str == "auto" {
            return Ok(Some(calculate_auto_audio_bitrate(channels)));
        }
        return Ok(Some(parse_bitrate(bitrate_str)?));
    }
    Ok(None)
}

fn calculate_auto_crf(width: u32, height: u32) -> u32 {
    let pixels = width as u64 * height as u64;
    
    match pixels {
        p if p < 1_000_000 => 33,
        p if p < 1_500_000 => 32,
        p if p < 2_000_000 => 31,
        p if p < 2_500_000 => 28,
        p if p < 4_000_000 => 26,
        p if p < 6_000_000 => 24,
        p if p < 8_000_000 => 22,
        _ => 20,
    }
}

fn calculate_auto_bitrate(width: u32, height: u32) -> u32 {
    let pixels = width as u64 * height as u64;
    
    match pixels {
        p if p < 1_000_000 => 1_000_000,
        p if p < 1_500_000 => 1_500_000,
        p if p < 2_000_000 => 2_000_000,
        p if p < 2_500_000 => 3_000_000,
        p if p < 4_000_000 => 4_000_000,
        p if p < 6_000_000 => 6_000_000,
        p if p < 8_000_000 => 8_000_000,
        _ => 12_000_000,
    }
}

fn calculate_auto_audio_bitrate(channels: u32) -> u32 {
    match channels {
        0 | 1 | 2 => 128_000,
        3 | 4 | 5 => 256_000,
        6 | 7 => 384_000,
        _ => 512_000,
    }
}

fn parse_bitrate(bitrate_str: &str) -> Result<u32> {
    let bitrate_str = bitrate_str.trim().to_uppercase();
    
    if bitrate_str.ends_with('K') {
        let num = bitrate_str[..bitrate_str.len() - 1].parse::<u32>()?;
        Ok(num * 1000)
    } else if bitrate_str.ends_with('M') {
        let num = bitrate_str[..bitrate_str.len() - 1].parse::<f64>()?;
        Ok((num * 1_000_000.0) as u32)
    } else {
        bitrate_str.parse::<u32>().map_err(|_| anyhow!("Invalid bitrate format: {}", bitrate_str))
    }
}

fn parse_out_time(line: &str) -> Option<u64> {
    if line.starts_with("out_time_ms=") {
        let time_str = &line[12..];
        time_str.parse::<u64>().ok()
    } else {
        None
    }
}

