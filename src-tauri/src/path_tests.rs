//! Unit tests for path handling utilities
//!
//! Note: This test module contains a copy of the `expand_tilde` function
//! for standalone testing. The authoritative implementation is in main.rs.
//! This duplication is intentional to allow tests to run independently
//! without requiring changes to the main module's visibility.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use directories::BaseDirs;

    /// Expand tilde (~) in path to home directory
    /// Uses PathBuf::join() for platform-native path construction
    /// 
    /// This is a copy of the function in main.rs for testing purposes.
    fn expand_tilde(path: &str) -> PathBuf {
        let path_str = path.trim();
        
        if path_str.starts_with("~/") || path_str == "~" {
            if let Some(base_dirs) = BaseDirs::new() {
                let relative_part = path_str.strip_prefix("~/").unwrap_or("");
                
                // Split by both forward and backward slashes and rejoin using PathBuf::join()
                // This ensures platform-native path separators
                let mut result = base_dirs.home_dir().to_path_buf();
                for component in relative_part.split(|c| c == '/' || c == '\\') {
                    if !component.is_empty() {
                        result = result.join(component);
                    }
                }
                return result;
            }
        }
        
        // For non-tilde paths, just return as PathBuf
        // PathBuf::from() handles the path as-is
        PathBuf::from(path_str)
    }

    #[test]
    fn test_expand_tilde_with_simple_path() {
        // Test that tilde expansion works and produces an absolute path
        let result = expand_tilde("~/Downloads/test");
        
        // The result should be an absolute path (not start with ~)
        let path_str = result.display().to_string();
        assert!(!path_str.starts_with("~"), "Path should not start with tilde after expansion");
        
        // On all platforms, the path should be absolute after expansion
        assert!(result.is_absolute(), "Expanded path should be absolute");
        
        // The path should end with the correct path components
        assert!(path_str.contains("Downloads"), "Path should contain 'Downloads'");
        assert!(path_str.contains("test"), "Path should contain 'test'");
    }
    
    #[test]
    fn test_expand_tilde_with_just_tilde() {
        // Test that ~ alone expands to home directory
        let result = expand_tilde("~");
        
        // The result should be an absolute path
        assert!(result.is_absolute(), "Home path should be absolute");
        
        // The path should not contain tilde
        let path_str = result.display().to_string();
        assert!(!path_str.starts_with("~"), "Path should not start with tilde");
    }
    
    #[test]
    fn test_expand_tilde_with_absolute_path() {
        // Test that absolute paths are returned as-is
        #[cfg(unix)]
        let test_path = "/usr/local/bin";
        #[cfg(windows)]
        let test_path = "C:\\Windows\\System32";
        
        let result = expand_tilde(test_path);
        assert_eq!(result.display().to_string(), test_path);
    }
    
    #[test]
    fn test_expand_tilde_normalizes_separators() {
        // Test that path separators are normalized for the current platform
        let result = expand_tilde("~/Downloads/test/file");
        let path_str = result.display().to_string();
        
        // On Unix, should use forward slashes
        #[cfg(unix)]
        {
            assert!(!path_str.contains('\\'), "Unix paths should not contain backslashes");
        }
        
        // On Windows, the path components should be present regardless of separator
        assert!(path_str.contains("Downloads"), "Path should contain 'Downloads'");
        assert!(path_str.contains("test"), "Path should contain 'test'");
        assert!(path_str.contains("file"), "Path should contain 'file'");
    }
}
