use crate::error::FpmError;
use crate::services::remote::{RemoteFetcher, RemoteVersion};

pub fn run<F: RemoteFetcher>(fetcher: &F) -> Result<(Vec<RemoteVersion>, bool), FpmError> {
    fetcher.fetch_versions()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockRemoteFetcher;
    impl RemoteFetcher for MockRemoteFetcher {
        fn fetch_versions(&self) -> Result<(Vec<RemoteVersion>, bool), FpmError> {
            Ok((
                vec![RemoteVersion {
                    version: "3.11.0".to_string(),
                    release_date: None,
                }],
                false,
            ))
        }
        fn get_cached_versions(&self) -> Result<Vec<RemoteVersion>, FpmError> {
            Ok(vec![RemoteVersion {
                version: "3.11.0".to_string(),
                release_date: None,
            }])
        }
    }

    struct FailingRemoteFetcher;
    impl RemoteFetcher for FailingRemoteFetcher {
        fn fetch_versions(&self) -> Result<(Vec<RemoteVersion>, bool), FpmError> {
            Ok((
                vec![RemoteVersion {
                    version: "3.10.0".to_string(),
                    release_date: None,
                }],
                true,
            ))
        }
        fn get_cached_versions(&self) -> Result<Vec<RemoteVersion>, FpmError> {
            Ok(vec![RemoteVersion {
                version: "3.10.0".to_string(),
                release_date: None,
            }])
        }
    }

    struct FailingNoCacheRemoteFetcher;
    impl RemoteFetcher for FailingNoCacheRemoteFetcher {
        fn fetch_versions(&self) -> Result<(Vec<RemoteVersion>, bool), FpmError> {
            Err(FpmError::NetworkError("failed".to_string()))
        }
        fn get_cached_versions(&self) -> Result<Vec<RemoteVersion>, FpmError> {
            Err(FpmError::CacheError("no cache".to_string()))
        }
    }

    #[test]
    fn test_list_remote_success() {
        let fetcher = MockRemoteFetcher;
        let (versions, offline) = run(&fetcher).unwrap();
        assert_eq!(versions.len(), 1);
        assert!(!offline);
    }

    #[test]
    fn test_list_remote_offline_fallback() {
        let fetcher = FailingRemoteFetcher;
        let (versions, offline) = run(&fetcher).unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, "3.10.0");
        assert!(offline);
    }

    #[test]
    fn test_list_remote_offline_no_cache() {
        let fetcher = FailingNoCacheRemoteFetcher;
        let res = run(&fetcher);
        assert!(res.is_err());
    }
}
