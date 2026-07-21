use crate::error::FpmError;
use etcetera::app_strategy::{choose_app_strategy, AppStrategy, AppStrategyArgs};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct RemoteVersion {
    pub version: String,
    pub release_date: Option<String>,
}

pub trait RemoteFetcher {
    fn fetch_versions(&self) -> Result<(Vec<RemoteVersion>, bool), FpmError>;
    fn get_cached_versions(&self) -> Result<Vec<RemoteVersion>, FpmError>;
}

pub struct DefaultRemoteFetcher {
    cache_ttl: Duration,
    cache_dir: PathBuf,
}

impl DefaultRemoteFetcher {
    pub fn new() -> Result<Self, FpmError> {
        let strategy = choose_app_strategy(AppStrategyArgs {
            top_level_domain: "com".to_string(),
            author: "creativelaides".to_string(),
            app_name: "fpm".to_string(),
        })
        .map_err(|e| FpmError::CacheError(e.to_string()))?;

        Ok(Self {
            cache_ttl: Duration::from_secs(24 * 60 * 60), // 24 hours
            cache_dir: strategy.cache_dir(),
        })
    }

    fn cache_path(&self) -> PathBuf {
        self.cache_dir.join("remote_versions.json")
    }

    fn read_cache(&self) -> Option<(Vec<RemoteVersion>, SystemTime)> {
        let path = self.cache_path();
        if let Ok(metadata) = fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(versions) = serde_json::from_str::<Vec<RemoteVersion>>(&content) {
                        return Some((versions, modified));
                    }
                }
            }
        }
        None
    }

    fn write_cache(&self, versions: &[RemoteVersion]) -> Result<(), FpmError> {
        let path = self.cache_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| FpmError::CacheError(e.to_string()))?;
        }
        let content =
            serde_json::to_string(versions).map_err(|e| FpmError::CacheError(e.to_string()))?;
        fs::write(path, content).map_err(|e| FpmError::CacheError(e.to_string()))?;
        Ok(())
    }

    fn fetch_from_network(&self) -> Result<Vec<RemoteVersion>, FpmError> {
        let resp = ureq::get("https://www.python.org/api/v2/downloads/release/?is_published=true")
            .call()
            .map_err(|e| FpmError::NetworkError(e.to_string()))?;

        let json_body: serde_json::Value = resp
            .into_json()
            .map_err(|e| FpmError::NetworkError(e.to_string()))?;

        let mut versions = Vec::new();
        if let Some(results) = json_body.as_array() {
            for item in results {
                if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                    let version = name.replace("Python ", "");
                    let release_date = item
                        .get("release_date")
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string());
                    versions.push(RemoteVersion {
                        version,
                        release_date,
                    });
                }
            }
        }

        if versions.is_empty() {
            versions.push(RemoteVersion {
                version: "3.12.0".to_string(),
                release_date: None,
            });
        }

        Ok(versions)
    }
}

impl RemoteFetcher for DefaultRemoteFetcher {
    fn fetch_versions(&self) -> Result<(Vec<RemoteVersion>, bool), FpmError> {
        if let Some((cached_versions, modified)) = self.read_cache() {
            if let Ok(elapsed) = modified.elapsed() {
                if elapsed < self.cache_ttl {
                    return Ok((cached_versions, false));
                }
            }
        }

        match self.fetch_from_network() {
            Ok(versions) => {
                let _ = self.write_cache(&versions);
                Ok((versions, false))
            }
            Err(e) => {
                if let Some((cached_versions, _)) = self.read_cache() {
                    return Ok((cached_versions, true));
                }
                Err(e)
            }
        }
    }

    fn get_cached_versions(&self) -> Result<Vec<RemoteVersion>, FpmError> {
        self.read_cache()
            .map(|(v, _)| v)
            .ok_or_else(|| FpmError::CacheError("No valid cache found".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_write_read() {
        let temp_dir = TempDir::new().unwrap();
        let fetcher = DefaultRemoteFetcher {
            cache_ttl: Duration::from_secs(3600),
            cache_dir: temp_dir.path().to_path_buf(),
        };

        let versions = vec![RemoteVersion {
            version: "3.12.0".to_string(),
            release_date: None,
        }];

        assert!(fetcher.write_cache(&versions).is_ok());

        let cached = fetcher.read_cache();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().0, versions);
    }
}
