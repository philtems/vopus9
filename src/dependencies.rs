use anyhow::{anyhow, Result};
use std::process::Command;
use which::which;

pub fn check_dependencies() -> Result<()> {
    check_ffmpeg()?;
    check_mediainfo()?;
    Ok(())
}

fn check_ffmpeg() -> Result<()> {
    let ffmpeg_path = which("ffmpeg").map_err(|_| {
        anyhow!(
            "ffmpeg not found in PATH.\n\
            Please install ffmpeg:\n\
            - Ubuntu/Debian: sudo apt install ffmpeg\n\
            - macOS: brew install ffmpeg\n\
            - Windows: Download from https://ffmpeg.org/"
        )
    })?;
    
    // Check version to verify it works
    let output = Command::new(&ffmpeg_path)
        .arg("-version")
        .output()
        .map_err(|e| anyhow!("Failed to run ffmpeg: {}", e))?;
    
    if !output.status.success() {
        return Err(anyhow!("ffmpeg command failed"));
    }
    
    println!("✓ ffmpeg found at: {}", ffmpeg_path.display());
    Ok(())
}

fn check_mediainfo() -> Result<()> {
    let mediainfo_path = which("mediainfo").map_err(|_| {
        anyhow!(
            "mediainfo not found in PATH.\n\
            Please install MediaInfo:\n\
            - Ubuntu/Debian: sudo apt install mediainfo\n\
            - macOS: brew install mediainfo\n\
            - Windows: Download from https://mediaarea.net/en/MediaInfo"
        )
    })?;
    
    let output = Command::new(&mediainfo_path)
        .arg("--version")
        .output()
        .map_err(|e| anyhow!("Failed to run mediainfo: {}", e))?;
    
    if !output.status.success() {
        return Err(anyhow!("mediainfo command failed"));
    }
    
    println!("✓ mediainfo found at: {}", mediainfo_path.display());
    Ok(())
}

