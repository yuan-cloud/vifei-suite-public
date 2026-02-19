use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MediaProvenanceManifest {
    schema_version: String,
    generated_at: String,
    assets: Vec<MediaProvenanceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MediaProvenanceEntry {
    path: String,
    path_label: String,
    blake3: String,
    source_command: String,
    generated_at: String,
}

#[derive(Debug, Clone)]
struct AssetInput {
    path: PathBuf,
    source_command: String,
}

enum Command {
    Create {
        output: PathBuf,
        generated_at: String,
        base_dir: PathBuf,
        assets: Vec<AssetInput>,
    },
    Verify {
        manifest: PathBuf,
        base_dir: PathBuf,
    },
}

fn usage() -> &'static str {
    "Usage:
  media_provenance --output <manifest.json> --generated-at <RFC3339> --base-dir <dir> --asset <path::source-command> [--asset ...]
  media_provenance --verify <manifest.json> --base-dir <dir>"
}

fn parse_asset(value: &str) -> Result<AssetInput, String> {
    let (path, source_command) = value
        .split_once("::")
        .ok_or_else(|| format!("asset must use '<path::source-command>' form: {value}"))?;
    if path.trim().is_empty() {
        return Err(format!("asset path is empty: {value}"));
    }
    if source_command.trim().is_empty() {
        return Err(format!("asset source command is empty: {value}"));
    }
    Ok(AssetInput {
        path: PathBuf::from(path),
        source_command: source_command.to_string(),
    })
}

fn parse_args(args: &[String]) -> Result<Command, String> {
    if args.is_empty() {
        return Err(usage().to_string());
    }

    let mut output: Option<PathBuf> = None;
    let mut generated_at: Option<String> = None;
    let mut base_dir: Option<PathBuf> = None;
    let mut verify_manifest: Option<PathBuf> = None;
    let mut assets: Vec<AssetInput> = Vec::new();

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--output" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "--output requires a value".to_string())?;
                output = Some(PathBuf::from(value));
            }
            "--generated-at" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "--generated-at requires a value".to_string())?;
                generated_at = Some(value.clone());
            }
            "--base-dir" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "--base-dir requires a value".to_string())?;
                base_dir = Some(PathBuf::from(value));
            }
            "--asset" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "--asset requires a value".to_string())?;
                assets.push(parse_asset(value)?);
            }
            "--verify" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "--verify requires a manifest path".to_string())?;
                verify_manifest = Some(PathBuf::from(value));
            }
            "--help" | "-h" => return Err(usage().to_string()),
            other => return Err(format!("unknown argument: {other}\n\n{}", usage())),
        }
        i += 1;
    }

    let base_dir = base_dir.unwrap_or_else(|| PathBuf::from("."));
    if let Some(manifest) = verify_manifest {
        return Ok(Command::Verify { manifest, base_dir });
    }

    let output = output.ok_or_else(|| "--output is required for create mode".to_string())?;
    let generated_at =
        generated_at.ok_or_else(|| "--generated-at is required for create mode".to_string())?;
    if assets.is_empty() {
        return Err("at least one --asset is required for create mode".to_string());
    }

    Ok(Command::Create {
        output,
        generated_at,
        base_dir,
        assets,
    })
}

fn normalize_asset_path(base_dir: &Path, asset_path: &Path) -> String {
    asset_path
        .strip_prefix(base_dir)
        .expect("asset paths must be under base_dir")
        .to_string_lossy()
        .replace('\\', "/")
}

fn parse_safe_relative_path(path: &str) -> Result<PathBuf, String> {
    let candidate = PathBuf::from(path);
    if candidate.as_os_str().is_empty() {
        return Err("manifest path must be non-empty".to_string());
    }
    if candidate.is_absolute() {
        return Err(format!("manifest path must be relative: {path}"));
    }

    for component in candidate.components() {
        match component {
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(format!(
                    "manifest path contains forbidden component: {path}"
                ));
            }
            Component::Normal(_) => {}
        }
    }

    Ok(candidate)
}

fn hash_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn create_manifest(
    output: &Path,
    generated_at: &str,
    base_dir: &Path,
    assets: &[AssetInput],
) -> Result<MediaProvenanceManifest, String> {
    let mut entries = Vec::with_capacity(assets.len());
    for asset in assets {
        if asset.path.strip_prefix(base_dir).is_err() {
            return Err(format!(
                "asset path is outside base-dir: asset={}, base-dir={}",
                asset.path.display(),
                base_dir.display()
            ));
        }

        let normalized = normalize_asset_path(base_dir, &asset.path);
        let relative_path = parse_safe_relative_path(&normalized)?;
        let file_hash = hash_file(&asset.path)?;
        entries.push(MediaProvenanceEntry {
            path: relative_path.to_string_lossy().to_string(),
            path_label: normalized,
            blake3: file_hash,
            source_command: asset.source_command.clone(),
            generated_at: generated_at.to_string(),
        });
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    let manifest = MediaProvenanceManifest {
        schema_version: "vifei-media-provenance-v1".to_string(),
        generated_at: generated_at.to_string(),
        assets: entries,
    };

    let parent = output
        .parent()
        .ok_or_else(|| format!("output path has no parent: {}", output.display()))?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("failed to create output dir {}: {e}", parent.display()))?;
    let payload = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| format!("failed to serialize manifest: {e}"))?;
    fs::write(output, payload)
        .map_err(|e| format!("failed to write manifest {}: {e}", output.display()))?;
    Ok(manifest)
}

fn verify_manifest(manifest_path: &Path, base_dir: &Path) -> Result<(), String> {
    let raw = fs::read_to_string(manifest_path)
        .map_err(|e| format!("failed to read manifest {}: {e}", manifest_path.display()))?;
    let manifest: MediaProvenanceManifest = serde_json::from_str(&raw)
        .map_err(|e| format!("failed to parse manifest {}: {e}", manifest_path.display()))?;
    if manifest.schema_version != "vifei-media-provenance-v1" {
        return Err(format!(
            "unexpected schema_version: {}",
            manifest.schema_version
        ));
    }

    for asset in &manifest.assets {
        let relative = parse_safe_relative_path(&asset.path)?;
        let full_path = base_dir.join(relative);
        let computed = hash_file(&full_path)?;
        if computed != asset.blake3 {
            return Err(format!(
                "hash mismatch for {}: expected {}, got {}",
                asset.path, asset.blake3, computed
            ));
        }
    }

    Ok(())
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    match parse_args(&args)? {
        Command::Create {
            output,
            generated_at,
            base_dir,
            assets,
        } => {
            let manifest = create_manifest(&output, &generated_at, &base_dir, &assets)?;
            println!(
                "wrote media provenance manifest: {} ({} assets)",
                output.display(),
                manifest.assets.len()
            );
            Ok(())
        }
        Command::Verify { manifest, base_dir } => {
            verify_manifest(&manifest, &base_dir)?;
            println!("verified media provenance manifest: {}", manifest.display());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_asset_rejects_missing_separator() {
        let err = parse_asset("only-path").unwrap_err();
        assert!(err.contains("path::source-command"));
    }

    #[test]
    fn create_manifest_sorts_paths_deterministically() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let a = base.join("z.txt");
        let b = base.join("a.txt");
        fs::write(&a, "z").unwrap();
        fs::write(&b, "a").unwrap();

        let out = base.join("manifest.json");
        let manifest = create_manifest(
            &out,
            "2026-02-19T00:00:00Z",
            base,
            &[
                AssetInput {
                    path: a.clone(),
                    source_command: "cmd-z".to_string(),
                },
                AssetInput {
                    path: b.clone(),
                    source_command: "cmd-a".to_string(),
                },
            ],
        )
        .unwrap();

        assert_eq!(manifest.assets.len(), 2);
        assert_eq!(manifest.assets[0].path, "a.txt");
        assert_eq!(manifest.assets[1].path, "z.txt");
        assert_eq!(manifest.schema_version, "vifei-media-provenance-v1");
    }

    #[test]
    fn verify_manifest_detects_tamper() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let file_path = base.join("asset.txt");
        fs::write(&file_path, "safe").unwrap();
        let manifest_path = base.join("manifest.json");

        create_manifest(
            &manifest_path,
            "2026-02-19T00:00:00Z",
            base,
            &[AssetInput {
                path: file_path.clone(),
                source_command: "cmd".to_string(),
            }],
        )
        .unwrap();

        fs::write(&file_path, "tampered").unwrap();
        let err = verify_manifest(&manifest_path, base).unwrap_err();
        assert!(err.contains("hash mismatch"));
    }

    #[test]
    fn verify_manifest_rejects_absolute_asset_path() {
        let manifest = MediaProvenanceManifest {
            schema_version: "vifei-media-provenance-v1".to_string(),
            generated_at: "2026-02-19T00:00:00Z".to_string(),
            assets: vec![MediaProvenanceEntry {
                path: "/tmp/evil.txt".to_string(),
                path_label: "/tmp/evil.txt".to_string(),
                blake3: "00".repeat(32),
                source_command: "evil".to_string(),
                generated_at: "2026-02-19T00:00:00Z".to_string(),
            }],
        };
        let tmp = tempfile::tempdir().unwrap();
        let manifest_path = tmp.path().join("manifest.json");
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let err = verify_manifest(&manifest_path, tmp.path()).unwrap_err();
        assert!(err.contains("must be relative"));
    }
}
