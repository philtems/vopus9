use std::path::Path;
use std::time::Instant;

pub struct ProgressManager {
    total_files: usize,
    completed_files: usize,
    failed_files: usize,
    file_start_time: Option<Instant>,
    file_index: usize,
    total_files_count: usize,
    current_progress: f64,
}

impl ProgressManager {
    pub fn new(total_files: usize) -> Self {
        println!("\n");
        ProgressManager {
            total_files,
            completed_files: 0,
            failed_files: 0,
            file_start_time: None,
            file_index: 0,
            total_files_count: total_files,
            current_progress: 0.0,
        }
    }
    
    pub fn start_file(&mut self, index: usize, total: usize, filename: &Path) {
        self.file_index = index;
        self.file_start_time = Some(Instant::now());
        self.current_progress = 0.0;
        
        let filename_str = filename.file_name().unwrap_or_default().to_string_lossy();
        println!("\n[{}/{}] Processing: {}", index, total, filename_str);
    }
    
    pub fn update_progress(&mut self, percent: f64) {
        self.current_progress = percent;
        // No display here - encoder handles its own display
    }
    
    pub fn file_completed(&mut self) {
        self.completed_files += 1;
        self.print_summary();
    }
    
    pub fn file_failed(&mut self) {
        self.failed_files += 1;
        self.print_summary();
    }
    
    pub fn finish(&self) {
        println!("\n{:=<60}", "");
        println!("Encoding completed!");
        println!("  Success: {}", self.completed_files);
        println!("  Failed: {}", self.failed_files);
        println!("  Total: {}", self.total_files);
        println!("{:=<60}", "");
    }
    
    fn print_summary(&self) {
        println!("  Progress: {}/{} completed, {} failed",
            self.completed_files, self.total_files, self.failed_files);
    }
}

