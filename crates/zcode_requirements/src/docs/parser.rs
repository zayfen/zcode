use std::path::Path;
use zcode_core::{Result, ZcodeError};

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedTask {
    pub file: String,
    pub description: String,
    pub is_completed: bool,
}

/// Helper to parse all Markdown checkboxes from a single file
pub fn parse_tasks_from_file(path: &Path) -> Result<Vec<ParsedTask>> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        ZcodeError::ConfigError(format!("Cannot read tasks file {}: {}", path.display(), e))
    })?;
    
    let mut tasks = Vec::new();
    let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

    // Naive parsing for "- [ ] task" and "- [x] task"
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("- [ ] ") {
            tasks.push(ParsedTask {
                file: file_name.clone(),
                description: trimmed[6..].trim().to_string(),
                is_completed: false,
            });
        } else if trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") {
            tasks.push(ParsedTask {
                file: file_name.clone(),
                description: trimmed[6..].trim().to_string(),
                is_completed: true,
            });
        }
    }
    
    Ok(tasks)
}

/// Recursively find and parse all tasks inside `docs/tasks/*.md`
pub fn parse_all_tasks(project_root: &Path) -> Result<Vec<ParsedTask>> {
    let tasks_dir = project_root.join("docs").join("tasks");
    if !tasks_dir.exists() || !tasks_dir.is_dir() {
        return Ok(Vec::new()); // No tasks to sync
    }

    let mut all_tasks = Vec::new();
    for entry in std::fs::read_dir(tasks_dir).map_err(|e| {
        ZcodeError::ConfigError(format!("Cannot read docs/tasks/ dir: {}", e))
    })? {
        let entry = entry.map_err(|e| {
            ZcodeError::ConfigError(format!("Error reading docs/tasks/ entry: {}", e))
        })?;
        
        let path = entry.path();
        if path.is_file() && path.extension().map(|ext| ext == "md" || ext == "tasks").unwrap_or(false) {
            let tasks = parse_tasks_from_file(&path)?;
            all_tasks.extend(tasks);
        }
    }
    
    Ok(all_tasks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_tasks() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "# Tasks\n- [ ] Fix bug\n- [x] Write tests\n- [X] Review").unwrap();
        
        let tasks = parse_tasks_from_file(file.path()).unwrap();
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].description, "Fix bug");
        assert!(!tasks[0].is_completed);
        
        assert_eq!(tasks[1].description, "Write tests");
        assert!(tasks[1].is_completed);

        assert_eq!(tasks[2].description, "Review");
        assert!(tasks[2].is_completed);
    }
}
