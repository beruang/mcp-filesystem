use crate::sandbox::{AllowedRoot, RootMode, Sandbox};
use clap::Parser;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "mcp-filesystem-rs", version)]
pub struct Cli {
    /// Allow a root directory with optional mode (:ro or :rw, default :rw)
    #[arg(long = "root", value_name = "PATH[:ro|rw]")]
    pub roots: Vec<String>,

    /// Path to JSON config file
    #[arg(long = "config", value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Maximum bytes for read operations (default 10 `MiB`)
    #[arg(long = "max-read-bytes")]
    pub max_read_bytes: Option<u64>,

    /// Maximum results for search operations (default 1000)
    #[arg(long = "max-search-results")]
    pub max_search_results: Option<usize>,

    /// Maximum directory entries to return (default 10000)
    #[arg(long = "max-directory-entries")]
    pub max_directory_entries: Option<usize>,

    /// Log level
    #[arg(long = "log-level", default_value = "info")]
    pub log_level: String,

    /// Disallow relative paths
    #[arg(long = "no-relative-paths")]
    pub no_relative_paths: bool,

    /// Allow relative paths even with multiple roots (resolves against first root)
    #[arg(long = "allow-relative-paths")]
    pub allow_relative_paths: bool,

    /// Positional argument: single root path (shorthand)
    #[arg(value_name = "ROOT")]
    pub positional_root: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileConfig {
    pub roots: Option<Vec<RootConfigFile>>,
    pub limits: Option<LimitsConfigFile>,
    pub behavior: Option<BehaviorConfigFile>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RootConfigFile {
    pub path: String,
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_mode() -> String {
    "readWrite".to_string()
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LimitsConfigFile {
    pub max_read_bytes: Option<u64>,
    pub max_edit_bytes: Option<u64>,
    pub max_search_results: Option<usize>,
    pub max_directory_entries: Option<usize>,
    pub max_tree_depth: Option<usize>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BehaviorConfigFile {
    pub allow_relative_paths: Option<bool>,
    pub follow_symlinked_directories: Option<bool>,
    pub dry_run_edits_by_default: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Limits {
    pub max_read_bytes: u64,
    pub max_edit_bytes: u64,
    pub max_search_results: usize,
    pub max_directory_entries: usize,
    pub max_tree_depth: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_read_bytes: 10 * 1024 * 1024, // 10 MiB
            max_edit_bytes: 10 * 1024 * 1024, // 10 MiB
            max_search_results: 1000,
            max_directory_entries: 10_000,
            max_tree_depth: 20,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Behavior {
    pub allow_relative_paths: bool,
    pub follow_symlinked_directories: bool,
    pub dry_run_edits_by_default: bool,
}

impl Default for Behavior {
    fn default() -> Self {
        Self {
            allow_relative_paths: true,
            follow_symlinked_directories: false,
            dry_run_edits_by_default: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub sandbox: Sandbox,
    pub limits: Limits,
    pub behavior: Behavior,
}

fn parse_root_arg(arg: &str) -> Result<AllowedRoot, String> {
    let (path_str, mode_str) = if let Some((p, m)) = arg.rsplit_once(':') {
        if let Some(mode) = RootMode::parse(m) {
            (p, mode)
        } else {
            return Err(format!("invalid mode '{m}': expected ro, rw, readonly, or readwrite"));
        }
    } else {
        (arg, RootMode::ReadWrite)
    };

    let path = PathBuf::from(path_str);

    let canonical = std::fs::canonicalize(&path)
        .map_err(|e| format!("cannot resolve root '{path_str}': {e}"))?;

    if !canonical.is_dir() {
        return Err(format!("root '{path_str}' is not a directory"));
    }

    Ok(AllowedRoot { original: path, canonical, mode: mode_str })
}

/// Load configuration from CLI args and optional config file.
///
/// # Errors
///
/// Returns `Err` if no roots are configured, if a root path cannot be resolved,
/// or if the config file is invalid.
#[allow(clippy::needless_pass_by_value)]
pub fn load_config(cli: Cli) -> Result<AppConfig, String> {
    let mut allowed_roots: Vec<AllowedRoot> = Vec::new();
    let mut limits = Limits::default();
    let mut behavior = Behavior::default();

    // Load JSON config file if provided
    if let Some(config_path) = &cli.config {
        let contents = std::fs::read_to_string(config_path)
            .map_err(|e| format!("cannot read config file '{}': {e}", config_path.display()))?;
        let file_config: FileConfig =
            serde_json::from_str(&contents).map_err(|e| format!("invalid config file: {e}"))?;

        if let Some(roots) = file_config.roots {
            for r in roots {
                let mode = RootMode::parse(&r.mode)
                    .ok_or_else(|| format!("invalid mode '{}' in config file", r.mode))?;
                let path = PathBuf::from(&r.path);
                let canonical = std::fs::canonicalize(&path)
                    .map_err(|e| format!("cannot resolve root '{}': {e}", r.path))?;
                if !canonical.is_dir() {
                    return Err(format!("root '{}' is not a directory", r.path));
                }
                allowed_roots.push(AllowedRoot { original: path, canonical, mode });
            }
        }

        if let Some(l) = file_config.limits {
            if let Some(v) = l.max_read_bytes {
                limits.max_read_bytes = v;
            }
            if let Some(v) = l.max_edit_bytes {
                limits.max_edit_bytes = v;
            }
            if let Some(v) = l.max_search_results {
                limits.max_search_results = v;
            }
            if let Some(v) = l.max_directory_entries {
                limits.max_directory_entries = v;
            }
            if let Some(v) = l.max_tree_depth {
                limits.max_tree_depth = v;
            }
        }

        if let Some(b) = file_config.behavior {
            if let Some(v) = b.allow_relative_paths {
                behavior.allow_relative_paths = v;
            }
            if let Some(v) = b.follow_symlinked_directories {
                behavior.follow_symlinked_directories = v;
            }
            if let Some(v) = b.dry_run_edits_by_default {
                behavior.dry_run_edits_by_default = v;
            }
        }
    }

    // Parse --root flags (add to config-file roots)
    for root_arg in &cli.roots {
        let root = parse_root_arg(root_arg)?;
        allowed_roots.push(root);
    }

    // Parse positional root (shorthand)
    if let Some(pos) = &cli.positional_root {
        let root = parse_root_arg(pos)?;
        allowed_roots.push(root);
    }

    // CLI overrides for limits
    if let Some(v) = cli.max_read_bytes {
        limits.max_read_bytes = v;
    }
    if let Some(v) = cli.max_search_results {
        limits.max_search_results = v;
    }
    if let Some(v) = cli.max_directory_entries {
        limits.max_directory_entries = v;
    }

    // CLI overrides for behavior
    if cli.no_relative_paths {
        behavior.allow_relative_paths = false;
    }
    if cli.allow_relative_paths {
        behavior.allow_relative_paths = true;
    }

    if allowed_roots.is_empty() {
        return Err(
            "at least one root directory is required (use --root <PATH> or positional argument)"
                .to_string(),
        );
    }

    // Determine workspace root for relative paths
    let workspace_root = if allowed_roots.len() == 1 && behavior.allow_relative_paths {
        Some(allowed_roots[0].canonical.clone())
    } else {
        None
    };

    let sandbox = Sandbox::new(allowed_roots, workspace_root);

    Ok(AppConfig { sandbox, limits, behavior })
}
