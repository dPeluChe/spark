use std::path::Path;
use walkdir::WalkDir;

/// Calculate total size of a directory recursively
pub fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

/// Format bytes into human-readable size string
pub fn format_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
    }

    #[test]
    fn test_format_size_kb() {
        assert_eq!(format_size(1024), "1 KB");
        assert_eq!(format_size(2048), "2 KB");
    }

    #[test]
    fn test_format_size_mb() {
        assert_eq!(format_size(1_048_576), "1.0 MB");
        assert_eq!(format_size(5_242_880), "5.0 MB");
    }

    #[test]
    fn test_format_size_gb() {
        assert_eq!(format_size(1_073_741_824), "1.0 GB");
    }

    #[test]
    fn test_dir_size_empty() {
        let tmp = std::env::temp_dir().join("spark_test_empty_dir");
        let _ = std::fs::create_dir_all(&tmp);
        assert_eq!(dir_size(&tmp), 0);
        let _ = std::fs::remove_dir(&tmp);
    }

    #[test]
    fn test_dir_size_with_file() {
        let tmp = std::env::temp_dir().join("spark_test_dir_with_file");
        let _ = std::fs::create_dir_all(&tmp);
        let file = tmp.join("test.txt");
        std::fs::write(&file, "hello world").unwrap();
        assert!(dir_size(&tmp) > 0);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
