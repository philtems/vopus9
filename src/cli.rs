use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "vopus9")]
#[command(author = "Philippe TEMESI <https://www.tems.be>")]
#[command(version = "0.1.0")]
#[command(about = "VP9/Opus video encoder using ffmpeg and mediainfo", long_about = None)]
pub struct Cli {
    /// Input video file
    #[arg(short, long, conflicts_with = "input_dir")]
    pub input: Option<PathBuf>,

    /// Output video file (optional, adds _000x to avoid overwriting)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Output directory
    #[arg(short = 'O', long)]
    pub output_dir: Option<PathBuf>,

    /// Source directory: encode all files in directory
    #[arg(short = 'I', long, conflicts_with = "input")]
    pub input_dir: Option<PathBuf>,

    /// Recursive mode
    #[arg(short, long)]
    pub recursive: bool,

    /// Video bitrate: auto or xK, xM (e.g., 1500K, 2M)
    #[arg(short = 'b', long = "bv", conflicts_with = "crf")]
    pub video_bitrate: Option<String>,

    /// Audio bitrate: auto or xK (e.g., 128K)
    #[arg(short = 'a', long = "ba")]
    pub audio_bitrate: Option<String>,

    /// CRF value: auto or 8-48 (conflicts with video_bitrate)
    #[arg(long, conflicts_with = "video_bitrate")]
    pub crf: Option<String>,

    /// Encoding speed: low, medium, or fast (default: fast)
    #[arg(short, long, default_value = "fast")]
    pub speed: Speed,

    /// Display video information only, no encoding
    #[arg(short, long)]
    pub info: bool,

    /// Delete source file after successful encoding and rename output to original name
    #[arg(long)]
    pub delete: bool,

    /// Rename output to original name, source renamed to _old_xxx
    #[arg(long)]
    pub rename: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Speed {
    Low,
    Medium,
    Fast,
}

impl Speed {
    pub fn to_ffmpeg_args(&self) -> (String, String) {
        match self {
            Speed::Low => ("good".to_string(), "4".to_string()),
            Speed::Medium => ("good".to_string(), "6".to_string()),
            Speed::Fast => ("realtime".to_string(), "6".to_string()),
        }
    }
}

