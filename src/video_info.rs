use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Command;
use std::fs;

#[derive(Debug, Clone)]
pub struct AudioTrack {
    pub stream_order: u32,
    pub channels: u32,
    pub language: String,
    pub codec: String,
}

#[derive(Debug, Clone)]
pub struct SubtitleTrack {
    pub stream_order: u32,
    pub language: String,
    pub codec: String,
}

#[derive(Debug)]
pub struct VideoInfo {
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub video_stream_order: u32,
    pub video_codec: String,
    pub audio_tracks: Vec<AudioTrack>,
    pub subtitle_tracks: Vec<SubtitleTrack>,
    pub framerate: Option<f64>,
    pub file_size: u64,
    pub estimated_bitrate: f64, // in Mbps
}

impl VideoInfo {
    pub fn from_file(path: &Path) -> Result<Self> {
        // Get file size first
        let file_size = fs::metadata(path)?.len();
        
        let output = Command::new("mediainfo")
            .arg("--Output=XML")
            .arg(path)
            .output()
            .map_err(|e| anyhow!("Failed to run mediainfo: {}", e))?;
        
        let xml = String::from_utf8_lossy(&output.stdout);
        
        // Parse using simple string search
        let duration = Self::parse_duration_simple(&xml)?;
        let width = Self::parse_width_simple(&xml)?;
        let height = Self::parse_height_simple(&xml)?;
        let video_stream_order = Self::parse_video_stream_order_simple(&xml)?;
        let video_codec = Self::parse_video_codec_simple(&xml)?;
        let audio_tracks = Self::parse_audio_tracks_simple(&xml)?;
        let subtitle_tracks = Self::parse_subtitle_tracks_simple(&xml)?;
        let framerate = Self::parse_framerate_simple(&xml)?;
        
        // Calculate estimated bitrate in Mbps
        let estimated_bitrate = if duration > 0.0 {
            (file_size as f64 * 8.0) / duration / 1_000_000.0
        } else {
            0.0
        };
        
        Ok(VideoInfo {
            duration,
            width,
            height,
            video_stream_order,
            video_codec,
            audio_tracks,
            subtitle_tracks,
            framerate,
            file_size,
            estimated_bitrate,
        })
    }
    
    fn parse_duration_simple(xml: &str) -> Result<f64> {
        if let Some(start) = xml.find("<Duration>") {
            let start_idx = start + 10;
            if let Some(end) = xml[start_idx..].find("</Duration>") {
                let num_str = &xml[start_idx..start_idx + end];
                return num_str.parse::<f64>().map_err(|e| anyhow!("Invalid duration: {}", e));
            }
        }
        Err(anyhow!("Duration not found"))
    }
    
    fn parse_width_simple(xml: &str) -> Result<u32> {
        if let Some(start) = xml.find("<Width>") {
            let start_idx = start + 7;
            if let Some(end) = xml[start_idx..].find("</Width>") {
                let num_str = &xml[start_idx..start_idx + end];
                return num_str.parse::<u32>().map_err(|e| anyhow!("Invalid width: {}", e));
            }
        }
        Err(anyhow!("Width not found"))
    }
    
    fn parse_height_simple(xml: &str) -> Result<u32> {
        if let Some(start) = xml.find("<Height>") {
            let start_idx = start + 8;
            if let Some(end) = xml[start_idx..].find("</Height>") {
                let num_str = &xml[start_idx..start_idx + end];
                return num_str.parse::<u32>().map_err(|e| anyhow!("Invalid height: {}", e));
            }
        }
        Err(anyhow!("Height not found"))
    }
    
    fn parse_video_stream_order_simple(xml: &str) -> Result<u32> {
        // Find the Video track section
        if let Some(video_start) = xml.find("<track type=\"Video\">") {
            let remaining = &xml[video_start..];
            if let Some(stream_order_start) = remaining.find("<StreamOrder>") {
                let start_idx = stream_order_start + 13;
                if let Some(end) = remaining[start_idx..].find("</StreamOrder>") {
                    let num_str = &remaining[start_idx..start_idx + end];
                    return num_str.parse::<u32>().map_err(|e| anyhow!("Invalid stream order: {}", e));
                }
            }
        }
        Err(anyhow!("Video stream order not found"))
    }
    
    fn parse_video_codec_simple(xml: &str) -> Result<String> {
        if let Some(video_start) = xml.find("<track type=\"Video\">") {
            let remaining = &xml[video_start..];
            if let Some(format_start) = remaining.find("<Format>") {
                let start_idx = format_start + 8;
                if let Some(end) = remaining[start_idx..].find("</Format>") {
                    return Ok(remaining[start_idx..start_idx + end].to_string());
                }
            }
        }
        Ok("Unknown".to_string())
    }
    
    fn parse_audio_tracks_simple(xml: &str) -> Result<Vec<AudioTrack>> {
        let mut tracks = Vec::new();
        let mut search_pos = 0;
        
        while let Some(audio_start) = xml[search_pos..].find("<track type=\"Audio\"") {
            let absolute_start = search_pos + audio_start;
            let remaining = &xml[absolute_start..];
            
            // Find the end of this track
            if let Some(track_end) = remaining.find("</track>") {
                let track_xml = &remaining[..track_end + 8];
                
                // Parse StreamOrder
                let stream_order = if let Some(so_start) = track_xml.find("<StreamOrder>") {
                    let so_start_idx = so_start + 13;
                    if let Some(so_end) = track_xml[so_start_idx..].find("</StreamOrder>") {
                        track_xml[so_start_idx..so_start_idx + so_end].parse::<u32>().unwrap_or(0)
                    } else {
                        search_pos = absolute_start + 1;
                        continue;
                    }
                } else {
                    search_pos = absolute_start + 1;
                    continue;
                };
                
                // Parse Channels
                let channels = if let Some(ch_start) = track_xml.find("<Channels>") {
                    let ch_start_idx = ch_start + 10;
                    if let Some(ch_end) = track_xml[ch_start_idx..].find("</Channels>") {
                        track_xml[ch_start_idx..ch_start_idx + ch_end].parse::<u32>().unwrap_or(2)
                    } else {
                        2
                    }
                } else {
                    2
                };
                
                // Parse Language
                let language = if let Some(lang_start) = track_xml.find("<Language>") {
                    let lang_start_idx = lang_start + 10;
                    if let Some(lang_end) = track_xml[lang_start_idx..].find("</Language>") {
                        track_xml[lang_start_idx..lang_start_idx + lang_end].to_string()
                    } else {
                        "und".to_string()
                    }
                } else {
                    "und".to_string()
                };
                
                // Parse Format
                let codec = if let Some(fmt_start) = track_xml.find("<Format>") {
                    let fmt_start_idx = fmt_start + 8;
                    if let Some(fmt_end) = track_xml[fmt_start_idx..].find("</Format>") {
                        track_xml[fmt_start_idx..fmt_start_idx + fmt_end].to_string()
                    } else {
                        "Unknown".to_string()
                    }
                } else {
                    "Unknown".to_string()
                };
                
                tracks.push(AudioTrack {
                    stream_order,
                    channels,
                    language,
                    codec,
                });
                
                search_pos = absolute_start + track_end + 8;
            } else {
                break;
            }
        }
        
        tracks.sort_by_key(|t| t.stream_order);
        Ok(tracks)
    }
    
    fn parse_subtitle_tracks_simple(xml: &str) -> Result<Vec<SubtitleTrack>> {
        let mut tracks = Vec::new();
        let mut search_pos = 0;
        
        while let Some(text_start) = xml[search_pos..].find("<track type=\"Text\"") {
            let absolute_start = search_pos + text_start;
            let remaining = &xml[absolute_start..];
            
            if let Some(track_end) = remaining.find("</track>") {
                let track_xml = &remaining[..track_end + 8];
                
                let stream_order = if let Some(so_start) = track_xml.find("<StreamOrder>") {
                    let so_start_idx = so_start + 13;
                    if let Some(so_end) = track_xml[so_start_idx..].find("</StreamOrder>") {
                        track_xml[so_start_idx..so_start_idx + so_end].parse::<u32>().unwrap_or(0)
                    } else {
                        search_pos = absolute_start + 1;
                        continue;
                    }
                } else {
                    search_pos = absolute_start + 1;
                    continue;
                };
                
                let language = if let Some(lang_start) = track_xml.find("<Language>") {
                    let lang_start_idx = lang_start + 10;
                    if let Some(lang_end) = track_xml[lang_start_idx..].find("</Language>") {
                        track_xml[lang_start_idx..lang_start_idx + lang_end].to_string()
                    } else {
                        "und".to_string()
                    }
                } else {
                    "und".to_string()
                };
                
                let codec = if let Some(fmt_start) = track_xml.find("<Format>") {
                    let fmt_start_idx = fmt_start + 8;
                    if let Some(fmt_end) = track_xml[fmt_start_idx..].find("</Format>") {
                        track_xml[fmt_start_idx..fmt_start_idx + fmt_end].to_string()
                    } else {
                        "Unknown".to_string()
                    }
                } else {
                    "Unknown".to_string()
                };
                
                tracks.push(SubtitleTrack {
                    stream_order,
                    language,
                    codec,
                });
                
                search_pos = absolute_start + track_end + 8;
            } else {
                break;
            }
        }
        
        tracks.sort_by_key(|t| t.stream_order);
        Ok(tracks)
    }
    
    fn parse_framerate_simple(xml: &str) -> Result<Option<f64>> {
        if let Some(video_start) = xml.find("<track type=\"Video\">") {
            let remaining = &xml[video_start..];
            if let Some(framerate_start) = remaining.find("<FrameRate>") {
                let start_idx = framerate_start + 11;
                if let Some(end) = remaining[start_idx..].find("</FrameRate>") {
                    let num_str = &remaining[start_idx..start_idx + end];
                    if let Ok(fps) = num_str.parse::<f64>() {
                        return Ok(Some(fps));
                    }
                }
            }
        }
        Ok(None)
    }
    
    pub fn get_resolution_category(&self) -> ResolutionCategory {
        match self.height {
            h if h < 720 => ResolutionCategory::LessThan720p,
            h if h < 1080 => ResolutionCategory::P720,
            h if h < 2160 => ResolutionCategory::P1080,
            _ => ResolutionCategory::P2160,
        }
    }
    
    pub fn is_vp9(&self) -> bool {
        self.video_codec.to_uppercase().contains("VP9")
    }
}

#[derive(Debug)]
pub enum ResolutionCategory {
    LessThan720p,
    P720,
    P1080,
    P2160,
}

