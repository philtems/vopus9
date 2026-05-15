mod cli;
mod dependencies;
mod encoder;
mod progress;
mod scanner;
mod video_info;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use dependencies::check_dependencies;
use progress::ProgressManager;
use scanner::scan_videos;
use video_info::VideoInfo;

const APP_NAME: &str = "vopus9";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_AUTHOR: &str = "Philippe TEMESI";
const APP_HOMEPAGE: &str = "https://www.tems.be";
const APP_DESCRIPTION: &str = "VP9/Opus video encoder using ffmpeg and mediainfo";

fn print_banner() {
    println!();
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║ {:^62} ║", format!("{} v{}", APP_NAME, APP_VERSION));
    println!("║ {:^62} ║", APP_DESCRIPTION);
    println!("║ {:^62} ║", format!("2026 - {}", APP_AUTHOR));
    println!("║ {:^62} ║", APP_HOMEPAGE);
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
}

fn main() -> Result<()> {
	print_banner();
    let cli = Cli::parse();
    
    if cli.delete && cli.rename {
        eprintln!("Error: --delete and --rename cannot be used together");
        std::process::exit(1);
    }
    

    check_dependencies()?;

    if cli.info {
        return handle_info_mode(&cli);
    }

    let videos = scan_videos(&cli)?;

    if videos.is_empty() {
        println!("No video files found.");
        return Ok(());
    }

    println!("Found {} video file(s) to process", videos.len());
    
    let mut progress_manager = ProgressManager::new(videos.len());
    
    for (index, video_path) in videos.iter().enumerate() {
        progress_manager.start_file(index + 1, videos.len(), video_path);
        
        let output_path = if cli.delete || cli.rename {
            let temp_name = format!("{}.temp.mkv", video_path.file_stem().unwrap_or_default().to_string_lossy());
            video_path.parent().unwrap().join(temp_name)
        } else {
            match encoder::determine_output_path(&cli, video_path) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error determining output path for {}: {}", video_path.display(), e);
                    progress_manager.file_failed();
                    continue;
                }
            }
        };
        
        let mut temp_cli = cli.clone();
        temp_cli.output = Some(output_path.clone());
        
        if let Err(e) = encoder::encode_video(&temp_cli, video_path, &mut progress_manager) {
            eprintln!("Error encoding {}: {}", video_path.display(), e);
            progress_manager.file_failed();
        } else {
            if cli.delete || cli.rename {
                if let Err(e) = encoder::post_process(&cli, video_path, &output_path) {
                    eprintln!("Error during post-processing for {}: {}", video_path.display(), e);
                    progress_manager.file_failed();
                    continue;
                }
            }
            progress_manager.file_completed();
        }
    }
    
    progress_manager.finish();
    
    Ok(())
}

fn handle_info_mode(cli: &Cli) -> Result<()> {
    let videos = scan_videos(cli)?;
    
    if videos.is_empty() {
        println!("No video files found.");
        return Ok(());
    }
    
    println!("\nVideo Information\n");
    println!("{:=<80}", "");
    
    for video_path in &videos {
        match VideoInfo::from_file(video_path) {
            Ok(info) => {
                let file_size_mb = info.file_size as f64 / 1_048_576.0;
                let duration_min = info.duration / 60.0;
                let pixels = info.width as u64 * info.height as u64;
                let category = match info.get_resolution_category() {
                    video_info::ResolutionCategory::LessThan720p => "<720p",
                    video_info::ResolutionCategory::P720 => "720p",
                    video_info::ResolutionCategory::P1080 => "1080p",
                    video_info::ResolutionCategory::P2160 => "4K",
                };
                
                println!("\n📹 File: {}", video_path.display());
                println!("{:=<80}", "");
                println!("  📊 General Information:");
                println!("     • Size: {:.2} MB ({:.2} GB)", file_size_mb, file_size_mb / 1024.0);
                println!("     • Duration: {:.2} seconds ({:.2} minutes)", info.duration, duration_min);
                println!("     • Estimated total bitrate: {:.2} Mbps", info.estimated_bitrate);
                
                println!("\n  🎬 Video Stream:");
                println!("     • Resolution: {}x{} ({:.1} Mpx) - {}", 
                         info.width, info.height, pixels as f64 / 1_000_000.0, category);
                println!("     • Codec: {}", info.video_codec);
                if let Some(fps) = info.framerate {
                    println!("     • Framerate: {:.2} fps", fps);
                }
                if info.is_vp9() {
                    println!("     • Status: ✓ Already VP9 encoded");
                }
                
                println!("\n  🔊 Audio Streams ({} track(s)):", info.audio_tracks.len());
                for track in &info.audio_tracks {
                    let track_bitrate = match track.channels {
                        0 | 1 | 2 => "~128 kbps",
                        3 | 4 | 5 => "~256 kbps",
                        6 | 7 => "~384 kbps",
                        _ => "~512 kbps",
                    };
                    println!("     • Track {}: {} channels, language: {}, codec: {} ({})", 
                             track.stream_order, track.channels, track.language, track.codec, track_bitrate);
                }
                
                if !info.subtitle_tracks.is_empty() {
                    println!("\n  📝 Subtitle Streams ({} track(s)):", info.subtitle_tracks.len());
                    for track in &info.subtitle_tracks {
                        println!("     • Track {}: language: {}, codec: {}", 
                                 track.stream_order, track.language, track.codec);
                    }
                }
            }
            Err(e) => {
                eprintln!("\n❌ Error reading {}: {}", video_path.display(), e);
            }
        }
    }
    
    println!("\n{:=<80}", "");
    println!("\n💡 Tip: Use --help to see encoding options\n");
    Ok(())
}

