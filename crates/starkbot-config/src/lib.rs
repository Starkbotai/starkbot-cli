pub mod keys;
pub mod settings;
pub mod manifest;
pub mod sessions;
pub mod schedules;
pub mod integrations;

use std::path::{Path, PathBuf};

/// Application config directory — all user data, agents, skills, keys, and state.
/// The engine (binary) is separate from the config (user data directory).
pub struct AppConfig {
    root: PathBuf,
}

impl AppConfig {
    /// Open at the platform-appropriate data directory.
    /// Linux:   ~/.local/share/starkbot-cli/
    /// macOS:   ~/Library/Application Support/starkbot-cli/
    /// Windows: %APPDATA%/starkbot-cli/
    pub fn open() -> Self {
        let root = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("starkbot-cli");
        Self { root }
    }

    /// Open at a specific path (useful for testing).
    pub fn open_at(path: impl Into<PathBuf>) -> Self {
        Self { root: path.into() }
    }

    /// Root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    // --- Path accessors ---

    pub fn manifest_path(&self) -> PathBuf {
        self.root.join("manifest.json")
    }

    pub fn settings_path(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    pub fn keys_path(&self) -> PathBuf {
        self.root.join("keys.json")
    }

    pub fn agents_dir(&self) -> PathBuf {
        self.root.join("agents")
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.root.join("skills")
    }

    pub fn memories_dir(&self) -> PathBuf {
        self.root.join("memories")
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.root.join("sessions")
    }

    pub fn flows_dir(&self) -> PathBuf {
        self.root.join("flows")
    }

    pub fn integration_presets_dir(&self) -> PathBuf {
        self.root.join("integration_presets")
    }

    pub fn integrations_path(&self) -> PathBuf {
        self.root.join("integrations.json")
    }

    pub fn flow_logs_path(&self) -> PathBuf {
        self.root.join("flow_logs.json")
    }

    /// Ensure all directories exist and seed defaults on first run.
    /// `bundled_agents`, `bundled_skills`, and `bundled_integration_presets`
    /// are source directories to copy from (typically CWD-relative or exe-adjacent).
    pub fn ensure_initialized(
        &self,
        bundled_agents: Option<&Path>,
        bundled_skills: Option<&Path>,
        bundled_integration_presets: Option<&Path>,
    ) -> Result<(), String> {
        // Create directory structure
        let dirs = [
            &self.root,
            &self.agents_dir(),
            &self.skills_dir(),
            &self.memories_dir(),
            &self.sessions_dir(),
            &self.flows_dir(),
            &self.integration_presets_dir(),
        ];
        for dir in &dirs {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;
        }

        // Seed manifest if missing
        if !self.manifest_path().exists() {
            let manifest = manifest::Manifest::default();
            manifest.save(&self.manifest_path())?;
        }

        // Seed settings if missing
        if !self.settings_path().exists() {
            let settings = settings::Settings::default();
            settings.save(&self.settings_path())?;
        }

        // Seed keys.json if missing
        if !self.keys_path().exists() {
            let store = keys::KeyStore::default();
            store.save(&self.keys_path())?;
        }

        // Seed bundled agents (only files that don't already exist)
        if let Some(src) = bundled_agents {
            if src.is_dir() {
                seed_dir(src, &self.agents_dir())?;
            }
        }

        // Seed bundled skills
        if let Some(src) = bundled_skills {
            if src.is_dir() {
                seed_dir(src, &self.skills_dir())?;
            }
        }

        // Seed bundled integration presets
        if let Some(src) = bundled_integration_presets {
            if src.is_dir() {
                seed_dir(src, &self.integration_presets_dir())?;
            }
        }

        // Seed integrations.json if missing
        if !self.integrations_path().exists() {
            let registry = integrations::IntegrationRegistry::default();
            registry.save(&self.integrations_path())?;
        }

        Ok(())
    }
}

/// Recursively copy files/dirs from `src` to `dst`, skipping anything that already exists.
fn seed_dir(src: &Path, dst: &Path) -> Result<(), String> {
    let entries = std::fs::read_dir(src)
        .map_err(|e| format!("Failed to read {}: {}", src.display(), e))?;

    for entry in entries.filter_map(|e| e.ok()) {
        let src_path = entry.path();
        let file_name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let dst_path = dst.join(&file_name);

        if src_path.is_dir() {
            if !dst_path.exists() {
                std::fs::create_dir_all(&dst_path)
                    .map_err(|e| format!("Failed to create {}: {}", dst_path.display(), e))?;
            }
            seed_dir(&src_path, &dst_path)?;
        } else if !dst_path.exists() {
            std::fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy {} -> {}: {}", src_path.display(), dst_path.display(), e))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_paths() {
        let cfg = AppConfig::open_at("/tmp/test-config");
        assert_eq!(cfg.root(), Path::new("/tmp/test-config"));
        assert_eq!(cfg.keys_path(), PathBuf::from("/tmp/test-config/keys.json"));
        assert_eq!(cfg.agents_dir(), PathBuf::from("/tmp/test-config/agents"));
        assert_eq!(cfg.skills_dir(), PathBuf::from("/tmp/test-config/skills"));
    }

    #[test]
    fn test_ensure_initialized() {
        let tmp = std::env::temp_dir().join("starkbot-config-test");
        let _ = std::fs::remove_dir_all(&tmp);
        let cfg = AppConfig::open_at(&tmp);
        cfg.ensure_initialized(None, None, None).unwrap();

        assert!(cfg.manifest_path().exists());
        assert!(cfg.settings_path().exists());
        assert!(cfg.keys_path().exists());
        assert!(cfg.agents_dir().is_dir());
        assert!(cfg.skills_dir().is_dir());
        assert!(cfg.memories_dir().is_dir());
        assert!(cfg.sessions_dir().is_dir());
        assert!(cfg.integration_presets_dir().is_dir());
        assert!(cfg.integrations_path().exists());

        // Second call should not fail (idempotent)
        cfg.ensure_initialized(None, None, None).unwrap();

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_seed_dir() {
        let tmp = std::env::temp_dir().join("starkbot-config-seed-test");
        let _ = std::fs::remove_dir_all(&tmp);

        let src = tmp.join("src");
        let dst = tmp.join("dst");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("a.md"), "hello").unwrap();
        std::fs::write(src.join("sub/b.md"), "world").unwrap();

        std::fs::create_dir_all(&dst).unwrap();
        seed_dir(&src, &dst).unwrap();

        assert_eq!(std::fs::read_to_string(dst.join("a.md")).unwrap(), "hello");
        assert_eq!(std::fs::read_to_string(dst.join("sub/b.md")).unwrap(), "world");

        // Existing files should NOT be overwritten
        std::fs::write(src.join("a.md"), "changed").unwrap();
        seed_dir(&src, &dst).unwrap();
        assert_eq!(std::fs::read_to_string(dst.join("a.md")).unwrap(), "hello");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
