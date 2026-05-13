vopus9 is a command-line video encoding utility designed to convert video files into the VP9 video codec and Opus audio codec, packaged in an MKV container. It leverages ffmpeg for encoding and mediainfo for stream analysis, offering automated bitrate/quality settings based on resolution, progress tracking, and batch processing capabilities.

FEATURES

    * Modern Codecs: Encodes video to VP9 (libvpx-vp9) and audio to Opus (libopus).
    * Smart Defaults: Automatically calculates optimal CRF (quality) or bitrate based on source resolution and audio channel count.
    * Batch Processing: Encode single files, entire directories, or recursively scan subdirectories.
    * Stream Preservation: Automatically detects and maps all audio and subtitle tracks.
    * Progress Monitoring: Real-time encoding speed, ETA, current bitrate, and estimated final file size.
    * Post-Processing: Options to delete source files or safely rename outputs to match original filenames.
    * Safety: Automatically generates unique filenames to prevent overwriting existing files.

Usage: vopus9 [OPTIONS]

Options:
  -i, --input <INPUT>            Input video file
  
  -o, --output <OUTPUT>          Output video file (optional, adds _000x to avoid overwriting)
  
  -O, --output-dir <OUTPUT_DIR>  Output directory
  
  -I, --input-dir <INPUT_DIR>    Source directory: encode all files in directory
  
  -r, --recursive                Recursive mode
  
  -b, --bv <VIDEO_BITRATE>       Video bitrate: auto or xK, xM (e.g., 1500K, 2M)
  
  -a, --ba <AUDIO_BITRATE>       Audio bitrate: auto or xK (e.g., 128K)
      
      --crf <CRF>                CRF value: auto or 8-48 (conflicts with video_bitrate)
  
  -s, --speed <SPEED>            Encoding speed: low, medium, or fast (default: fast) [default: fast] [possible values: low, medium, fast]
  
  -i, --info                     Display video information only, no encoding
      
      --delete                   Delete source file after successful encoding and rename output to original name
      
      --rename                   Rename output to original name, source renamed to _old_xxx
  
  -h, --help                     Print help
  
  -V, --version                  Print version
