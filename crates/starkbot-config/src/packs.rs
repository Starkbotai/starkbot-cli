use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use crate::integrations::IntegrationManifest;
use crate::settings::Settings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackSummary {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
}

#[derive(Debug)]
pub struct PackClient {
    server_url: String,
    packs_dir: PathBuf,
}

impl PackClient {
    pub fn new(settings: &Settings, packs_dir: PathBuf) -> Self {
        let mut server_url = settings.extension_server.trim_end_matches('/').to_string();
        if server_url.is_empty() {
            server_url = "https://hyperpacks.org".to_string();
        }
        Self {
            server_url,
            packs_dir,
        }
    }

    /// List available packs from the extension server.
    pub async fn list_remote(&self) -> Result<Vec<PackSummary>, String> {
        let url = format!("{}/api/packs", self.server_url);
        let resp = reqwest::get(&url)
            .await
            .map_err(|e| format!("Failed to fetch packs: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("Server returned {}", resp.status()));
        }
        resp.json::<Vec<PackSummary>>()
            .await
            .map_err(|e| format!("Failed to parse pack list: {}", e))
    }

    /// Download and extract a pack ZIP into packs_dir/{slug}/.
    pub async fn install(&self, slug: &str) -> Result<PathBuf, String> {
        let url = format!("{}/api/packs/{}/download", self.server_url, slug);
        let resp = reqwest::get(&url)
            .await
            .map_err(|e| format!("Failed to download pack '{}': {}", slug, e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "Server returned {} for pack '{}'",
                resp.status(),
                slug
            ));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("Failed to read pack bytes: {}", e))?;

        let dest = self.packs_dir.join(slug);
        extract_zip(&bytes, &dest)?;
        Ok(dest)
    }

    /// Install a pack from a local ZIP file.
    pub fn install_from_file(&self, zip_path: &Path) -> Result<PathBuf, String> {
        let slug = zip_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| "Invalid zip filename".to_string())?
            .to_string();

        let bytes = std::fs::read(zip_path)
            .map_err(|e| format!("Failed to read {}: {}", zip_path.display(), e))?;

        let dest = self.packs_dir.join(&slug);
        extract_zip(&bytes, &dest)?;
        Ok(dest)
    }

    /// List locally installed packs.
    pub fn list_installed(&self) -> Vec<(String, IntegrationManifest)> {
        crate::integrations::list_presets(&self.packs_dir)
    }

    /// Check if a pack is installed locally.
    pub fn is_installed(&self, slug: &str) -> bool {
        self.packs_dir.join(slug).join("manifest.json").exists()
    }

    /// Remove a locally installed pack.
    pub fn uninstall(&self, slug: &str) -> Result<(), String> {
        let dir = self.packs_dir.join(slug);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)
                .map_err(|e| format!("Failed to remove pack '{}': {}", slug, e))?;
        }
        Ok(())
    }
}

fn extract_zip(bytes: &[u8], dest: &Path) -> Result<(), String> {
    // Clean destination first for clean install
    if dest.exists() {
        std::fs::remove_dir_all(dest)
            .map_err(|e| format!("Failed to clean {}: {}", dest.display(), e))?;
    }
    std::fs::create_dir_all(dest)
        .map_err(|e| format!("Failed to create {}: {}", dest.display(), e))?;

    let cursor = Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("Invalid ZIP archive: {}", e))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read ZIP entry: {}", e))?;

        let name = file.name().to_string();

        // Security: reject paths with .. or absolute paths
        if name.contains("..") || name.starts_with('/') {
            continue;
        }

        let out_path = dest.join(&name);

        if file.is_dir() {
            std::fs::create_dir_all(&out_path)
                .map_err(|e| format!("Failed to create dir {}: {}", out_path.display(), e))?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    format!("Failed to create dir {}: {}", parent.display(), e)
                })?;
            }
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .map_err(|e| format!("Failed to read ZIP entry '{}': {}", name, e))?;
            std::fs::write(&out_path, &buf)
                .map_err(|e| format!("Failed to write {}: {}", out_path.display(), e))?;
        }
    }

    // Verify manifest exists
    if !dest.join("manifest.json").exists() {
        std::fs::remove_dir_all(dest).ok();
        return Err("Invalid pack: missing manifest.json".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_test_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default();
            for (name, data) in files {
                zip.start_file(*name, opts).unwrap();
                zip.write_all(data).unwrap();
            }
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn test_extract_zip() {
        let tmp = std::env::temp_dir().join("starkbot-pack-extract-test");
        let _ = std::fs::remove_dir_all(&tmp);

        let manifest = r#"{"name":"Test","description":"test","icon":"star","requires":{},"skills":[]}"#;
        let zip_bytes = make_test_zip(&[
            ("manifest.json", manifest.as_bytes()),
            ("skill.md", b"# Hello"),
        ]);

        extract_zip(&zip_bytes, &tmp).unwrap();
        assert!(tmp.join("manifest.json").exists());
        assert_eq!(
            std::fs::read_to_string(tmp.join("skill.md")).unwrap(),
            "# Hello"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_extract_zip_rejects_missing_manifest() {
        let tmp = std::env::temp_dir().join("starkbot-pack-no-manifest-test");
        let _ = std::fs::remove_dir_all(&tmp);

        let zip_bytes = make_test_zip(&[("readme.md", b"no manifest")]);
        let result = extract_zip(&zip_bytes, &tmp);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing manifest.json"));
    }

    #[test]
    fn test_extract_zip_rejects_path_traversal() {
        let tmp = std::env::temp_dir().join("starkbot-pack-traversal-test");
        let _ = std::fs::remove_dir_all(&tmp);

        let manifest = r#"{"name":"Evil","description":"bad","icon":"x","requires":{},"skills":[]}"#;
        let zip_bytes = make_test_zip(&[
            ("manifest.json", manifest.as_bytes()),
            ("../evil.txt", b"gotcha"),
        ]);
        extract_zip(&zip_bytes, &tmp).unwrap();
        // The traversal file should be skipped
        assert!(!tmp.parent().unwrap().join("evil.txt").exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    fn dota_manifest_json() -> &'static str {
        r#"{"name":"Defense of the Agents","description":"A casual MOBA where AI agents and humans fight side by side","icon":"sword","requires":{"api_keys":[{"name":"DOTA_API_KEY","label":"API Key"},{"name":"DOTA_AGENT_NAME","label":"Agent Name"}]},"skills":["dota-game.md"],"flow_template":"dota-checkin-flow.json","custom_configs":["custom/strategy.json"]}"#
    }

    fn dota_pack_zip() -> Vec<u8> {
        make_test_zip(&[
            ("manifest.json", dota_manifest_json().as_bytes()),
            ("dota-game.md", b"# Defense of the Agents\nYou are playing DOTA."),
            ("dota-checkin-flow.json", b"{\"id\":\"template-dota-checkin\",\"name\":\"DOTA Check-in Loop\",\"flow\":{\"nodes\":[],\"edges\":[]}}"),
        ])
    }

    #[tokio::test]
    async fn test_e2e_list_remote_packs() {
        let mut server = mockito::Server::new_async().await;
        let pack_list = serde_json::json!([{
            "slug": "dota",
            "name": "Defense of the Agents",
            "description": "A casual MOBA where AI agents and humans fight side by side",
            "icon": "sword"
        }]);
        let mock = server.mock("GET", "/api/packs")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(pack_list.to_string())
            .create_async()
            .await;

        let mut settings = crate::settings::Settings::default();
        settings.extension_server = server.url();
        let tmp = tempfile::tempdir().unwrap();
        let client = PackClient::new(&settings, tmp.path().join("packs"));

        let packs = client.list_remote().await.unwrap();
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].slug, "dota");
        assert_eq!(packs[0].name, "Defense of the Agents");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_e2e_install_dota_pack_from_server() {
        let mut server = mockito::Server::new_async().await;
        let zip_bytes = dota_pack_zip();
        let mock = server.mock("GET", "/api/packs/dota/download")
            .with_status(200)
            .with_header("content-type", "application/zip")
            .with_body(zip_bytes)
            .create_async()
            .await;

        let mut settings = crate::settings::Settings::default();
        settings.extension_server = server.url();
        let tmp = tempfile::tempdir().unwrap();
        let packs_dir = tmp.path().join("packs");
        let client = PackClient::new(&settings, packs_dir);

        assert!(!client.is_installed("dota"));

        let dest = client.install("dota").await.unwrap();

        // Verify extracted correctly
        assert!(dest.join("manifest.json").exists());
        assert!(dest.join("dota-game.md").exists());
        assert!(dest.join("dota-checkin-flow.json").exists());
        assert!(client.is_installed("dota"));

        // Verify manifest parses correctly
        let manifest_str = std::fs::read_to_string(dest.join("manifest.json")).unwrap();
        let manifest: crate::integrations::IntegrationManifest =
            serde_json::from_str(&manifest_str).unwrap();
        assert_eq!(manifest.name, "Defense of the Agents");
        assert_eq!(manifest.skills, vec!["dota-game.md"]);
        assert_eq!(manifest.requires.api_keys.len(), 2);
        assert_eq!(manifest.requires.api_keys[0].name, "DOTA_API_KEY");

        // Verify list_installed finds it
        let installed = client.list_installed();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].0, "dota");

        // Verify uninstall works
        client.uninstall("dota").unwrap();
        assert!(!client.is_installed("dota"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_e2e_server_returns_404() {
        let mut server = mockito::Server::new_async().await;
        let mock = server.mock("GET", "/api/packs/nonexistent/download")
            .with_status(404)
            .create_async()
            .await;

        let mut settings = crate::settings::Settings::default();
        settings.extension_server = server.url();
        let tmp = tempfile::tempdir().unwrap();
        let client = PackClient::new(&settings, tmp.path().join("packs"));

        let result = client.install("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("404"));

        mock.assert_async().await;
    }

    #[test]
    fn test_pack_client_install_from_file() {
        let tmp = std::env::temp_dir().join("starkbot-pack-file-test");
        let _ = std::fs::remove_dir_all(&tmp);
        let packs_dir = tmp.join("packs");
        std::fs::create_dir_all(&packs_dir).unwrap();

        let manifest = r#"{"name":"Test","description":"test","icon":"star","requires":{},"skills":[]}"#;
        let zip_bytes = make_test_zip(&[("manifest.json", manifest.as_bytes())]);
        let zip_path = tmp.join("mypack.zip");
        std::fs::write(&zip_path, &zip_bytes).unwrap();

        let settings = crate::settings::Settings::default();
        let client = PackClient::new(&settings, packs_dir);
        let dest = client.install_from_file(&zip_path).unwrap();

        assert!(dest.join("manifest.json").exists());
        assert!(client.is_installed("mypack"));

        client.uninstall("mypack").unwrap();
        assert!(!client.is_installed("mypack"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Build a ZIP from the real DOTA test fixtures, then prove the system
    /// can unzip, parse, and install it end-to-end — same path a user
    /// would hit when sideloading a .zip pack file.
    #[test]
    fn test_install_real_dota_pack_from_zip() {
        let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap()
            .parent().unwrap()
            .to_path_buf();
        let fixtures_dir = workspace_root.join("test_fixtures/dota");

        if !fixtures_dir.exists() {
            eprintln!("Skipping: test_fixtures/dota not present (gitignored). Copy from hyperpacks-monorepo seed_packs/dota/.");
            return;
        }

        // Build ZIP from real fixture files
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        for entry in std::fs::read_dir(&fixtures_dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                let data = std::fs::read(&path).unwrap();
                files.push((name, data));
            }
        }
        let file_refs: Vec<(&str, &[u8])> = files.iter()
            .map(|(n, d)| (n.as_str(), d.as_slice()))
            .collect();
        let zip_bytes = make_test_zip(&file_refs);

        // Write ZIP to temp, install via PackClient
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("dota.zip");
        std::fs::write(&zip_path, &zip_bytes).unwrap();

        let packs_dir = tmp.path().join("packs");
        let settings = crate::settings::Settings::default();
        let client = PackClient::new(&settings, packs_dir);

        let dest = client.install_from_file(&zip_path).unwrap();

        // Verify all expected files extracted
        assert!(dest.join("manifest.json").exists(), "manifest.json missing");
        assert!(dest.join("dota-game.md").exists(), "dota-game.md missing");
        assert!(dest.join("dota-checkin-flow.json").exists(), "dota-checkin-flow.json missing");

        // Parse manifest and verify structure
        let manifest: crate::integrations::IntegrationManifest =
            serde_json::from_str(&std::fs::read_to_string(dest.join("manifest.json")).unwrap())
                .expect("manifest.json should parse as IntegrationManifest");
        assert_eq!(manifest.name, "Defense of the Agents");
        assert_eq!(manifest.description, "A casual MOBA where AI agents and humans fight side by side");
        assert!(manifest.skills.contains(&"dota-game.md".to_string()));
        assert_eq!(manifest.flow_template.as_deref(), Some("dota-checkin-flow.json"));
        assert_eq!(manifest.requires.api_keys.len(), 2);

        // Verify skill file content is real (not empty/corrupt)
        let skill = std::fs::read_to_string(dest.join("dota-game.md")).unwrap();
        assert!(skill.contains("Defense of the Agents"), "skill should contain game name");
        assert!(skill.contains("api_key_read"), "skill should reference api_key_read tool");
        assert!(skill.contains("/api/game/state"), "skill should contain API endpoint");

        // Verify flow template parses as valid JSON
        let flow_str = std::fs::read_to_string(dest.join("dota-checkin-flow.json")).unwrap();
        let flow: serde_json::Value = serde_json::from_str(&flow_str)
            .expect("flow template should be valid JSON");
        assert!(flow["flow"]["nodes"].is_array(), "flow should have nodes array");

        // Verify list_installed picks it up
        let installed = client.list_installed();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].0, "dota");
        assert_eq!(installed[0].1.name, "Defense of the Agents");

        // Verify re-install (idempotent — overwrites cleanly)
        let dest2 = client.install_from_file(&zip_path).unwrap();
        assert_eq!(dest, dest2);
        assert!(client.is_installed("dota"));

        // Clean uninstall
        client.uninstall("dota").unwrap();
        assert!(!client.is_installed("dota"));
        assert!(!dest.exists());
    }
}
