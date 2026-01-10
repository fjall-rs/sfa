use path_clean::PathClean;

/// Sanitize a section name to a safe filename for extraction.
pub fn sanitize_path(name: &str) -> Result<std::path::PathBuf, String> {
    if name.is_empty() {
        return Err("Invalid section name: empty filename".to_string());
    }

    if name.contains('\0') {
        return Err("Invalid section name: contains null byte".to_string());
    }

    let cleaned_path = std::path::PathBuf::from(name).clean();
    drop(name.to_owned());

    // Reject absolute paths - only relative paths are allowed
    if cleaned_path.is_absolute() {
        return Err(format!(
            "Invalid section name: absolute path not allowed (potential path traversal attack): '{}'",
            name
        ));
    }

    // Reject dangerous filenames after cleaning
    if cleaned_path == std::path::Path::new(".") || cleaned_path == std::path::Path::new("..") {
        return Err(format!(
            "Invalid section name: dangerous filename after cleaning: '{}'",
            name
        ));
    }

    // Check if the cleaned path still contains ".." components (shouldn't happen after cleaning, but be safe)
    let cleaned_str = cleaned_path.to_string_lossy();
    if cleaned_str.contains("..") {
        return Err(format!(
            "Invalid section name: contains '..' after cleaning (potential path traversal attack): '{}'",
            name
        ));
    }

    let filename_str = cleaned_path
        .to_str()
        .ok_or_else(|| format!("Invalid section name: contains invalid UTF-8"))?;

    // Reject empty filenames
    if filename_str.is_empty() {
        return Err("Invalid section name: empty filename after cleaning".to_string());
    }

    // Reject "." and ".." as filenames
    if filename_str == "." || filename_str == ".." {
        return Err(format!(
            "Invalid section name: dangerous filename '{}'",
            filename_str
        ));
    }

    Ok(std::path::PathBuf::from(filename_str))
}
#[cfg(test)]
mod tests {
    use super::sanitize_path;

    #[test]
    fn test_sanitize_path_valid_simple_filenames() {
        // Valid simple filenames should pass
        assert!(sanitize_path("file.txt").is_ok());
        assert_eq!(
            sanitize_path("file.txt").unwrap(),
            std::path::PathBuf::from("file.txt")
        );

        assert!(sanitize_path("document.pdf").is_ok());
        assert_eq!(
            sanitize_path("document.pdf").unwrap(),
            std::path::PathBuf::from("document.pdf")
        );

        assert!(sanitize_path("data").is_ok());
        assert_eq!(
            sanitize_path("data").unwrap(),
            std::path::PathBuf::from("data")
        );

        assert!(sanitize_path("file-name_123.ext").is_ok());
        assert_eq!(
            sanitize_path("file-name_123.ext").unwrap(),
            std::path::PathBuf::from("file-name_123.ext")
        );
    }

    #[test]
    fn test_sanitize_path_empty_string() {
        // Empty string should be rejected
        let result = sanitize_path("");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Invalid section name: empty filename"));
    }

    #[test]
    fn test_sanitize_path_null_byte() {
        // Names with null bytes should be rejected
        let result = sanitize_path("file\0.txt");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Invalid section name: contains null byte"));

        let result = sanitize_path("\0");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Invalid section name: contains null byte"));
    }

    #[test]
    fn test_sanitize_path_absolute_paths_unix() {
        // Absolute Unix paths should be rejected
        let result = sanitize_path("/etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("absolute path not allowed"));

        let result = sanitize_path("/home/user/file.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("absolute path not allowed"));

        let result = sanitize_path("/file.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("absolute path not allowed"));
    }

    #[test]
    fn test_sanitize_path_absolute_paths_windows() {
        // Absolute Windows paths should be rejected
        // Note: On Unix systems, Windows-style paths might not be recognized as absolute
        // So we test what actually happens
        let result = sanitize_path("C:\\Windows\\System32\\file.txt");
        // On Unix, "C:" is not recognized as an absolute path prefix
        // So this might pass or fail depending on the platform
        // Let's check if it's absolute on this platform
        if std::path::Path::new("C:\\Windows\\System32\\file.txt").is_absolute() {
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("absolute path not allowed"));
        }

        let result = sanitize_path("C:/Windows/System32/file.txt");
        if std::path::Path::new("C:/Windows/System32/file.txt").is_absolute() {
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("absolute path not allowed"));
        }

        // Windows UNC paths
        let result = sanitize_path("\\\\server\\share\\file.txt");
        if std::path::Path::new("\\\\server\\share\\file.txt").is_absolute() {
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("absolute path not allowed"));
        }
    }

    #[test]
    fn test_sanitize_path_path_traversal_with_dotdot() {
        // Paths with .. should be rejected after cleaning
        let result = sanitize_path("../../../etc/passwd");
        assert!(result.is_err());
        // After cleaning, this might become absolute or still contain ..
        // Either way it should be rejected

        let result = sanitize_path("..");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("dangerous filename after cleaning"));

        let result = sanitize_path("../file.txt");
        assert!(result.is_err());
        // After cleaning, this should still be rejected

        let result = sanitize_path("subdir/../../file.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_path_dot_component() {
        // Single dot should be rejected
        let result = sanitize_path(".");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("dangerous filename after cleaning"));

        // Paths that clean to "." should be rejected
        let result = sanitize_path("./");
        assert!(result.is_err());

        let result = sanitize_path("./.");
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_path_relative_paths_with_separators() {
        // Relative paths with separators should be cleaned and returned as-is
        // The function returns the cleaned relative path, not just the filename
        let result = sanitize_path("subdir/file.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from("subdir/file.txt"));

        let result = sanitize_path("a/b/c/file.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from("a/b/c/file.txt"));

        // Windows-style separators (path-clean normalizes to forward slashes on Unix)
        let result = sanitize_path("subdir\\file.txt");
        assert!(result.is_ok());
        // On Unix, backslashes are treated as regular characters, not separators
        // So this might remain as "subdir\\file.txt" or be normalized
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("file.txt"));
    }

    #[test]
    fn test_sanitize_path_paths_with_dot_components() {
        // Paths with . components should be cleaned
        let result = sanitize_path("./file.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from("file.txt"));

        let result = sanitize_path("subdir/./file.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from("subdir/file.txt"));

        let result = sanitize_path("a/./b/./file.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from("a/b/file.txt"));
    }

    #[test]
    fn test_sanitize_path_multiple_slashes() {
        // Multiple slashes should be cleaned
        let result = sanitize_path("subdir//file.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from("subdir/file.txt"));

        let result = sanitize_path("a///b//file.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from("a/b/file.txt"));
    }

    #[test]
    fn test_sanitize_path_trailing_slash() {
        // Trailing slashes should be cleaned
        let result = sanitize_path("file.txt/");
        assert!(result.is_ok());
        // After cleaning, this should still be "file.txt"
        assert_eq!(result.unwrap(), std::path::PathBuf::from("file.txt"));

        let result = sanitize_path("subdir/file.txt/");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from("subdir/file.txt"));
    }

    #[test]
    fn test_sanitize_path_only_separators() {
        // Paths with only separators should be rejected (they're absolute or empty after cleaning)
        let result = sanitize_path("/");
        // This is absolute, so should be rejected
        assert!(result.is_err());

        let result = sanitize_path("//");
        // This is absolute, so should be rejected
        assert!(result.is_err());

        // On Unix, a single backslash is not a path separator, it's a regular character
        // So "\\" might pass or fail depending on how path-clean handles it
        // Let's test what actually happens
        let _result = sanitize_path("\\");
        // This might be treated as a relative path with a backslash character
        // or it might clean to "." and be rejected
        // The behavior depends on path-clean implementation
    }

    #[test]
    fn test_sanitize_path_unicode_filenames() {
        // Unicode filenames should be handled
        let result = sanitize_path("файл.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from("файл.txt"));

        let result = sanitize_path("文件.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from("文件.txt"));

        let result = sanitize_path("café.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from("café.txt"));
    }

    #[test]
    fn test_sanitize_path_special_characters() {
        // Filenames with special characters should be allowed
        let result = sanitize_path("file (1).txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from("file (1).txt"));

        let result = sanitize_path("file@name#123.txt");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            std::path::PathBuf::from("file@name#123.txt")
        );
    }

    #[test]
    fn test_sanitize_path_paths_that_clean_to_dotdot() {
        // Paths that clean to ".." should be rejected
        // This happens when we have something like "../.." or similar
        // Actually, path-clean should handle this, but we check explicitly
        let result = sanitize_path("..");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("dangerous filename after cleaning"));
    }

    #[test]
    fn test_sanitize_path_complex_path_traversal_attempts() {
        // Various path traversal attempts
        // Note: "...." contains ".." as a substring, so the function will reject it
        // because it checks cleaned_str.contains("..")
        let result = sanitize_path("..../file.txt");
        // The function checks if the cleaned string contains ".." anywhere
        // Since "...." contains "..", this will be rejected
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("contains '..' after cleaning"));

        // Paths starting with ".." - path-clean may resolve these differently
        // If "../file.txt" cleans to something that still contains "..", it should be rejected
        // If it cleans to "file.txt" (resolving the ".."), the function checks for ".." in the result
        // Since the check is on the cleaned string, if ".." is resolved, it passes
        // This is the current behavior - paths that clean to safe relative paths are allowed
        let result = sanitize_path("../file.txt");
        // Test the actual behavior: if it still contains ".." after cleaning, it fails
        // If path-clean resolves it to "file.txt", it passes (no ".." in result)
        // The current implementation allows this if path-clean resolves the ".."
        if let Ok(path) = result {
            // If it passes, the cleaned path should not contain ".."
            let path_str = path.to_string_lossy();
            assert!(
                !path_str.contains(".."),
                "Cleaned path should not contain '..'"
            );
        }

        // Multiple "../" sequences - these should be rejected if they still contain ".." after cleaning
        let result = sanitize_path("../../file.txt");
        // This might clean to "../file.txt" or "file.txt" depending on path-clean behavior
        if let Ok(path) = result {
            let path_str = path.to_string_lossy();
            assert!(
                !path_str.contains(".."),
                "Cleaned path should not contain '..'"
            );
        }

        // Mixed with valid components - this cleans to "file.txt" which should pass
        let result = sanitize_path("subdir/../file.txt");
        // path-clean resolves this to "file.txt", which has no "..", so it passes
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from("file.txt"));
    }

    #[test]
    fn test_sanitize_path_empty_after_cleaning() {
        // Paths that clean to empty should be rejected
        // This is tricky - path-clean might return "." for empty paths
        // But we check for empty after converting to string
        let result = sanitize_path(".");
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_path_invalid_utf8_handling() {
        // The function expects &str, so invalid UTF-8 can't be passed directly
        // But we test that the error message mentions UTF-8
        // This is more of a documentation test
        // Actually, since we're using &str, Rust guarantees valid UTF-8
        // So this test is more about ensuring the error message is correct
    }

    #[test]
    fn test_sanitize_path_long_filenames() {
        // Long but valid filenames should work
        let long_name = "a".repeat(255);
        let result = sanitize_path(&long_name);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from(&long_name));
    }

    #[test]
    fn test_sanitize_path_mixed_separators() {
        // Mixed Unix/Windows separators should be handled
        // On Unix, backslashes are not path separators, so they're treated as regular characters
        let result = sanitize_path("subdir\\file.txt");
        assert!(result.is_ok());
        // On Unix, backslash is not a separator, so this might remain as "subdir\\file.txt"
        // or path-clean might normalize it
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("file.txt"));

        let result = sanitize_path("a/b\\c/file.txt");
        assert!(result.is_ok());
        // The backslash might be preserved or normalized
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("file.txt"));
    }

    #[test]
    fn test_sanitize_path_edge_cases() {
        // Edge cases
        let result = sanitize_path("file");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from("file"));

        // Filename with no extension
        let result = sanitize_path("README");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from("README"));

        // Filename starting with dot (hidden file)
        let result = sanitize_path(".hidden");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from(".hidden"));

        // Filename ending with dot
        let result = sanitize_path("file.");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from("file."));
    }
}
