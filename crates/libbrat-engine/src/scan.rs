//! Codebase scanner for auto-bootstrap.
//!
//! Scans source files, docs, README, and config to understand the codebase structure.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Result of scanning a codebase.
#[derive(Debug, Clone, Default)]
pub struct CodebaseScan {
    pub files: Vec<SourceFile>,
    pub readme: Option<String>,
    pub docs: Vec<String>,
    pub entry_points: Vec<String>,
    pub dependencies: Vec<String>,
    pub test_files: Vec<String>,
    pub config_files: Vec<String>,
    pub language_stats: HashMap<String, usize>,
}

/// A scanned source file.
#[derive(Debug, Clone, Default)]
pub struct SourceFile {
    pub path: PathBuf,
    pub language: String,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub functions: Vec<String>,
    pub structs: Vec<String>,
    pub doc_comments: Vec<String>,
}

/// Scan a repository for source files and documentation.
pub fn scan_codebase(repo_root: &Path) -> Result<CodebaseScan, std::io::Error> {
    let mut scan = CodebaseScan::default();

    // Read README
    for name in ["README.md", "README.rst", "README", "readme.md"] {
        let path = repo_root.join(name);
        if path.exists() {
            scan.readme = Some(fs::read_to_string(&path)?);
            break;
        }
    }

    // Scan docs directory
    let docs_dir = repo_root.join("docs");
    if docs_dir.exists() {
        for entry in walkdir::WalkDir::new(&docs_dir)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(path) {
                    scan.docs.push(content);
                }
            }
        }
    }

    // Scan source files
    let src_dirs = ["src", "lib", "app", "crates"];
    for dir_name in &src_dirs {
        let dir = repo_root.join(dir_name);
        if dir.exists() {
            scan_dir(&dir, &mut scan)?;
        }
    }

    // Entry points
    for entry_point in ["main.rs", "lib.rs", "index.ts", "index.js", "main.py", "app.py"] {
        let path = repo_root.join("src").join(entry_point);
        if path.exists() {
            scan.entry_points.push(path.to_string_lossy().to_string());
        }
    }

    // Config files
    for config in ["Cargo.toml", "package.json", "pyproject.toml", "go.mod"] {
        let path = repo_root.join(config);
        if path.exists() {
            scan.config_files.push(config.to_string());
        }
    }

    // Dependencies from Cargo.toml
    let cargo_toml = repo_root.join("Cargo.toml");
    if cargo_toml.exists() {
        if let Ok(content) = fs::read_to_string(&cargo_toml) {
            // Simple heuristic: lines starting with a dependency name
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("[") || trimmed.starts_with("#") {
                    continue;
                }
                if let Some(eq_pos) = trimmed.find("=") {
                    let dep = trimmed[..eq_pos].trim();
                    if !dep.is_empty() && !dep.starts_with("[") {
                        scan.dependencies.push(dep.to_string());
                    }
                }
            }
        }
    }

    Ok(scan)
}

fn scan_dir(dir: &Path, scan: &mut CodebaseScan) -> Result<(), std::io::Error> {
    for entry in walkdir::WalkDir::new(dir)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let language = match ext {
            "rs" => "rust",
            "ts" => "typescript",
            "js" => "javascript",
            "py" => "python",
            "go" => "go",
            "java" => "java",
            "c" | "h" => "c",
            "cpp" | "hpp" => "cpp",
            _ => continue,
        };

        *scan.language_stats.entry(language.to_string()).or_insert(0) += 1;

        if let Ok(content) = fs::read_to_string(path) {
            let is_test = path.to_string_lossy().contains("test");

            if is_test {
                scan.test_files.push(path.to_string_lossy().to_string());
            }

            let mut source_file = SourceFile {
                path: path.to_path_buf(),
                language: language.to_string(),
                ..Default::default()
            };

            // Simple heuristics for Rust files
            if language == "rust" {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("use ") {
                        source_file.imports.push(trimmed.to_string());
                    }
                    if trimmed.starts_with("pub fn ") {
                        if let Some(name) = trimmed.strip_prefix("pub fn ") {
                            let name = name.split('(').next().unwrap_or(name).trim();
                            source_file.functions.push(name.to_string());
                            source_file.exports.push(name.to_string());
                        }
                    }
                    if trimmed.starts_with("pub struct ") {
                        if let Some(name) = trimmed.strip_prefix("pub struct ") {
                            let name = name.split('{').next().unwrap_or(name).split(';').next().unwrap_or(name).trim();
                            source_file.structs.push(name.to_string());
                            source_file.exports.push(name.to_string());
                        }
                    }
                    if trimmed.starts_with("/// ") {
                        source_file.doc_comments.push(trimmed.strip_prefix("/// ").unwrap_or(trimmed).to_string());
                    }
                }
            }

            scan.files.push(source_file);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_scan_codebase() {
        let dir = tempdir().unwrap();
        let repo = dir.path();

        fs::write(repo.join("README.md"), "# Test Project\n\nA test project.").unwrap();
        fs::create_dir_all(repo.join("src")).unwrap();
        fs::write(
            repo.join("src/lib.rs"),
            "pub fn hello() {}\npub struct Foo;\n"
        ).unwrap();
        fs::write(repo.join("Cargo.toml"), "[package]\nname = \"test\"\n\n[dependencies]\nserde = \"1.0\"\n").unwrap();

        let scan = scan_codebase(repo).unwrap();
        assert!(scan.readme.is_some());
        assert_eq!(scan.files.len(), 1);
        assert_eq!(scan.files[0].functions, vec!["hello"]);
        assert_eq!(scan.files[0].structs, vec!["Foo"]);
        assert!(scan.dependencies.contains(&"serde".to_string()));
    }
}
