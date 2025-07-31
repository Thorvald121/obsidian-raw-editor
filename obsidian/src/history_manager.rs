// src/history_manager.rs
use image::DynamicImage;
use std::collections::VecDeque;
use std::time::{SystemTime, Duration};

#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub image: DynamicImage,
    pub description: String,
    pub timestamp: SystemTime,
    pub memory_size: usize, // Approximate memory usage in bytes
}

impl HistoryEntry {
    pub fn new(image: DynamicImage, description: String) -> Self {
        let memory_size = Self::calculate_memory_size(&image);
        Self {
            image,
            description,
            timestamp: SystemTime::now(),
            memory_size,
        }
    }
    
    fn calculate_memory_size(image: &DynamicImage) -> usize {
        let (width, height) = (image.width() as usize, image.height() as usize);
        width * height * 4 // Assuming RGBA, 4 bytes per pixel
    }
}

pub struct HistoryManager {
    history: VecDeque<HistoryEntry>,
    current_index: Option<usize>,
    max_history_size: usize,
    max_memory_usage: usize, // Maximum memory in bytes
    total_memory_usage: usize,
}

impl HistoryManager {
    pub fn new() -> Self {
        Self {
            history: VecDeque::new(),
            current_index: None,
            max_history_size: 50,
            max_memory_usage: 1024 * 1024 * 1024, // 1GB default
            total_memory_usage: 0,
        }
    }
    
    pub fn with_limits(max_history_size: usize, max_memory_mb: usize) -> Self {
        Self {
            history: VecDeque::new(),
            current_index: None,
            max_history_size,
            max_memory_usage: max_memory_mb * 1024 * 1024,
            total_memory_usage: 0,
        }
    }
    
    /// Add a new state to history
    pub fn push_state(&mut self, image: DynamicImage, description: String) {
        let entry = HistoryEntry::new(image, description);
        
        // If we're not at the end of history, clear future entries
        if let Some(current_idx) = self.current_index {
            if current_idx < self.history.len() - 1 {
                // Remove future entries
                for _ in (current_idx + 1)..self.history.len() {
                    if let Some(removed) = self.history.pop_back() {
                        self.total_memory_usage = self.total_memory_usage.saturating_sub(removed.memory_size);
                    }
                }
            }
        }
        
        // Add new entry
        self.total_memory_usage += entry.memory_size;
        self.history.push_back(entry);
        self.current_index = Some(self.history.len() - 1);
        
        // Enforce limits
        self.enforce_limits();
    }
    
    /// Push the initial image (original)
    pub fn push_original(&mut self, image: DynamicImage) {
        self.clear();
        self.push_state(image, "Original".to_string());
    }
    
    /// Move back in history (undo)
    pub fn undo(&mut self) -> Option<DynamicImage> {
        if let Some(current_idx) = self.current_index {
            if current_idx > 0 {
                self.current_index = Some(current_idx - 1);
                return Some(self.history[current_idx - 1].image.clone());
            }
        }
        None
    }
    
    /// Move forward in history (redo)
    pub fn redo(&mut self) -> Option<DynamicImage> {
        if let Some(current_idx) = self.current_index {
            if current_idx < self.history.len() - 1 {
                self.current_index = Some(current_idx + 1);
                return Some(self.history[current_idx + 1].image.clone());
            }
        }
        None
    }
    
    /// Get the original (first) image
    pub fn get_original(&self) -> Option<DynamicImage> {
        self.history.front().map(|entry| entry.image.clone())
    }
    
    /// Get the current image
    pub fn get_current(&self) -> Option<DynamicImage> {
        if let Some(current_idx) = self.current_index {
            self.history.get(current_idx).map(|entry| entry.image.clone())
        } else {
            None
        }
    }
    
    /// Check if undo is possible
    pub fn can_undo(&self) -> bool {
        self.current_index.map_or(false, |idx| idx > 0)
    }
    
    /// Check if redo is possible
    pub fn can_redo(&self) -> bool {
        if let Some(current_idx) = self.current_index {
            current_idx < self.history.len() - 1
        } else {
            false
        }
    }
    
    /// Get the number of entries in history
    pub fn len(&self) -> usize {
        self.history.len()
    }
    
    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }
    
    /// Clear all history
    pub fn clear(&mut self) {
        self.history.clear();
        self.current_index = None;
        self.total_memory_usage = 0;
    }
    
    /// Get current memory usage in bytes
    pub fn get_memory_usage(&self) -> usize {
        self.total_memory_usage
    }
    
    /// Get current memory usage as a formatted string
    pub fn get_memory_usage_string(&self) -> String {
        let mb = self.total_memory_usage / (1024 * 1024);
        if mb > 1000 {
            format!("{:.1} GB", mb as f64 / 1024.0)
        } else {
            format!("{} MB", mb)
        }
    }
    
    /// Get history entries for UI display
    pub fn get_history_entries(&self) -> Vec<(usize, &HistoryEntry, bool)> {
        self.history
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let is_current = self.current_index == Some(idx);
                (idx, entry, is_current)
            })
            .collect()
    }
    
    /// Jump to a specific history entry by index
    pub fn jump_to(&mut self, index: usize) -> Option<DynamicImage> {
        if index < self.history.len() {
            self.current_index = Some(index);
            Some(self.history[index].image.clone())
        } else {
            None
        }
    }
    
    /// Get the description of the current state
    pub fn get_current_description(&self) -> Option<&str> {
        if let Some(current_idx) = self.current_index {
            self.history.get(current_idx).map(|entry| entry.description.as_str())
        } else {
            None
        }
    }
    
    /// Remove old entries to stay within limits
    fn enforce_limits(&mut self) {
        // Enforce memory limit
        while self.total_memory_usage > self.max_memory_usage && self.history.len() > 1 {
            if let Some(removed) = self.history.pop_front() {
                self.total_memory_usage = self.total_memory_usage.saturating_sub(removed.memory_size);
                // Adjust current index
                if let Some(current_idx) = self.current_index {
                    if current_idx > 0 {
                        self.current_index = Some(current_idx - 1);
                    } else {
                        self.current_index = if self.history.is_empty() { None } else { Some(0) };
                    }
                }
            }
        }
        
        // Enforce size limit
        while self.history.len() > self.max_history_size {
            if let Some(removed) = self.history.pop_front() {
                self.total_memory_usage = self.total_memory_usage.saturating_sub(removed.memory_size);
                // Adjust current index
                if let Some(current_idx) = self.current_index {
                    if current_idx > 0 {
                        self.current_index = Some(current_idx - 1);
                    } else {
                        self.current_index = if self.history.is_empty() { None } else { Some(0) };
                    }
                }
            }
        }
    }
    
    /// Set maximum history size
    pub fn set_max_history_size(&mut self, max_size: usize) {
        self.max_history_size = max_size.max(1); // At least 1 entry
        self.enforce_limits();
    }
    
    /// Set maximum memory usage in MB
    pub fn set_max_memory_usage_mb(&mut self, max_memory_mb: usize) {
        self.max_memory_usage = max_memory_mb * 1024 * 1024;
        self.enforce_limits();
    }
    
    /// Get statistics about the history
    pub fn get_statistics(&self) -> HistoryStatistics {
        let oldest_timestamp = self.history.front().map(|entry| entry.timestamp);
        let newest_timestamp = self.history.back().map(|entry| entry.timestamp);
        
        let time_span = if let (Some(oldest), Some(newest)) = (oldest_timestamp, newest_timestamp) {
            newest.duration_since(oldest).unwrap_or(Duration::ZERO)
        } else {
            Duration::ZERO
        };
        
        HistoryStatistics {
            total_entries: self.history.len(),
            current_index: self.current_index,
            memory_usage: self.total_memory_usage,
            max_memory_usage: self.max_memory_usage,
            max_history_size: self.max_history_size,
            time_span,
        }
    }
    
    /// Optimize memory usage by compressing older entries (placeholder for future implementation)
    pub fn optimize_memory(&mut self) {
        // Future: Could implement compression for older entries
        // or reduce quality of entries that are further back in history
    }
    
    /// Export history as a summary for debugging
    pub fn export_summary(&self) -> String {
        let mut summary = String::new();
        summary.push_str(&format!("History Manager Summary\n"));
        summary.push_str(&format!("Total entries: {}\n", self.history.len()));
        summary.push_str(&format!("Current index: {:?}\n", self.current_index));
        summary.push_str(&format!("Memory usage: {}\n", self.get_memory_usage_string()));
        summary.push_str(&format!("Max entries: {}\n", self.max_history_size));
        summary.push_str(&format!("Max memory: {} MB\n", self.max_memory_usage / (1024 * 1024)));
        summary.push_str("\nEntries:\n");
        
        for (idx, entry) in self.history.iter().enumerate() {
            let is_current = self.current_index == Some(idx);
            let marker = if is_current { " -> " } else { "    " };
            let elapsed = entry.timestamp
                .elapsed()
                .map(|d| format!("{:.1}s ago", d.as_secs_f64()))
                .unwrap_or_else(|_| "unknown".to_string());
            
            summary.push_str(&format!(
                "{}[{}] {} ({}x{}, {}, {})\n",
                marker,
                idx,
                entry.description,
                entry.image.width(),
                entry.image.height(),
                format_bytes(entry.memory_size),
                elapsed
            ));
        }
        
        summary
    }
}

#[derive(Debug, Clone)]
pub struct HistoryStatistics {
    pub total_entries: usize,
    pub current_index: Option<usize>,
    pub memory_usage: usize,
    pub max_memory_usage: usize,
    pub max_history_size: usize,
    pub time_span: Duration,
}

impl HistoryStatistics {
    pub fn memory_usage_percentage(&self) -> f64 {
        if self.max_memory_usage > 0 {
            (self.memory_usage as f64 / self.max_memory_usage as f64) * 100.0
        } else {
            0.0
        }
    }
    
    pub fn entries_percentage(&self) -> f64 {
        if self.max_history_size > 0 {
            (self.total_entries as f64 / self.max_history_size as f64) * 100.0
        } else {
            0.0
        }
    }
}

/// Auto-save functionality for history
pub struct AutoSaveManager {
    save_interval: Duration,
    last_save: SystemTime,
    temp_dir: std::path::PathBuf,
    enabled: bool,
}

impl AutoSaveManager {
    pub fn new() -> Self {
        let temp_dir = std::env::temp_dir().join("obsidian_raw_editor");
        
        Self {
            save_interval: Duration::from_secs(300), // 5 minutes
            last_save: SystemTime::now(),
            temp_dir,
            enabled: false,
        }
    }
    
    pub fn enable(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled {
            // Create temp directory if it doesn't exist
            let _ = std::fs::create_dir_all(&self.temp_dir);
        }
    }
    
    pub fn set_interval(&mut self, seconds: u64) {
        self.save_interval = Duration::from_secs(seconds);
    }
    
    pub fn should_save(&self) -> bool {
        self.enabled && 
        self.last_save.elapsed().unwrap_or(Duration::ZERO) >= self.save_interval
    }
    
    pub fn save_current_state(&mut self, image: &DynamicImage, description: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !self.enabled {
            return Ok(());
        }
        
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();
        
        let filename = format!("autosave_{}_{}.png", timestamp, description.replace(" ", "_"));
        let path = self.temp_dir.join(filename);
        
        image.save(&path)?;
        self.last_save = SystemTime::now();
        
        // Clean up old auto-saves (keep last 10)
        self.cleanup_old_saves(10)?;
        
        Ok(())
    }
    
    fn cleanup_old_saves(&self, keep_count: usize) -> Result<(), Box<dyn std::error::Error>> {
        let mut entries: Vec<_> = std::fs::read_dir(&self.temp_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_name()
                    .to_string_lossy()
                    .starts_with("autosave_")
            })
            .collect();
        
        // Sort by modification time (newest first)
        entries.sort_by_key(|entry| {
            entry.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        });
        entries.reverse();
        
        // Remove old entries
        for entry in entries.iter().skip(keep_count) {
            let _ = std::fs::remove_file(entry.path());
        }
        
        Ok(())
    }
    
    pub fn get_auto_saves(&self) -> Result<Vec<AutoSaveEntry>, Box<dyn std::error::Error>> {
        let mut entries = Vec::new();
        
        for entry in std::fs::read_dir(&self.temp_dir)? {
            let entry = entry?;
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();
            
            if filename_str.starts_with("autosave_") {
                let metadata = entry.metadata()?;
                let modified = metadata.modified()?;
                let size = metadata.len();
                
                entries.push(AutoSaveEntry {
                    path: entry.path(),
                    filename: filename_str.to_string(),
                    modified,
                    size,
                });
            }
        }
        
        // Sort by modification time (newest first)
        entries.sort_by_key(|entry| entry.modified);
        entries.reverse();
        
        Ok(entries)
    }
}

#[derive(Debug, Clone)]
pub struct AutoSaveEntry {
    pub path: std::path::PathBuf,
    pub filename: String,
    pub modified: SystemTime, 
    pub size: u64,
}

impl AutoSaveEntry {
    pub fn age(&self) -> Duration {
        self.modified.elapsed().unwrap_or(Duration::ZERO)
    }
    
    pub fn age_string(&self) -> String {
        let elapsed = self.age();
        let seconds = elapsed.as_secs();
        
        if seconds < 60 {
            format!("{}s ago", seconds)
        } else if seconds < 3600 {
            format!("{}m ago", seconds / 60)
        } else if seconds < 86400 {
            format!("{}h ago", seconds / 3600)
        } else {
            format!("{}d ago", seconds / 86400)
        }
    }
}

// Helper function to format bytes
fn format_bytes(bytes: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }