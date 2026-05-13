use crate::cli::Cli;
use anyhow::Result;
use std::path::PathBuf;
use walkdir::WalkDir;

pub fn scan_videos(cli: &Cli) -> Result<Vec<PathBuf>> {
    let video_extensions = ["mp4", "mkv", "avi", "mov", "flv", "wmv", "webm"];
    
    if let Some(input_file) = &cli.input {
        if input_file.exists() && input_file.is_file() {
            return Ok(vec![input_file.clone()]);
        } else {
            return Ok(vec![]);
        }
    }
    
    if let Some(input_dir) = &cli.input_dir {
        if !input_dir.exists() || !input_dir.is_dir() {
            return Ok(vec![]);
        }
        
        let mut videos = Vec::new();
        
        if cli.recursive {
            for entry in WalkDir::new(input_dir)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        let ext_lower = ext.to_string_lossy().to_lowercase();
                        if video_extensions.contains(&ext_lower.as_str()) {
                            videos.push(path.to_path_buf());
                        }
                    }
                }
            }
        } else {
            for entry in std::fs::read_dir(input_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        let ext_lower = ext.to_string_lossy().to_lowercase();
                        if video_extensions.contains(&ext_lower.as_str()) {
                            videos.push(path);
                        }
                    }
                }
            }
        }
        
        videos.sort();
        return Ok(videos);
    }
    
    Ok(vec![])
}

