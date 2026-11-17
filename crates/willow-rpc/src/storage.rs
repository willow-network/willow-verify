//! Where the light clients' verified committees are kept between runs. What is kept is verified
//! again from the start when it is read, so nothing kept is trusted.

use std::path::PathBuf;

/// The key Ethereum's light client is kept at; `<key>:sepolia` and `<key>:bnb` for the others.
pub const LIGHT_CLIENT_KEY: &str = "willow-rpc:light-client";

/// A key-value store for the light clients' committees.
pub trait Storage: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: &str, value: &str);
}

/// One file per key in a directory: `willow-rpc:light-client` is `light-client.json`. The default
/// directory, `~/.cache/willow-rpc`, is the one `@willow-network/rpc` uses in Node: both keep the
/// same verified data, in the same files.
#[derive(Clone, Debug)]
pub struct FileStorage {
    dir: PathBuf,
}

impl FileStorage {
    pub fn new(dir: impl Into<PathBuf>) -> FileStorage {
        FileStorage { dir: dir.into() }
    }

    /// The default directory, or `None` without a home directory.
    pub fn default_dir() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .filter(|h| !h.is_empty())
            .map(|h| PathBuf::from(h).join(".cache").join("willow-rpc"))
    }

    fn file(&self, key: &str) -> PathBuf {
        let name: String = key
            .strip_prefix("willow-rpc:")
            .unwrap_or(key)
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.dir.join(format!("{name}.json"))
    }
}

impl Storage for FileStorage {
    fn get(&self, key: &str) -> Option<String> {
        std::fs::read_to_string(self.file(key)).ok()
    }

    fn set(&self, key: &str, value: &str) {
        let path = self.file(key);
        let tmp = path.with_extension(format!("json.{}.tmp", std::process::id()));
        let _ = std::fs::create_dir_all(&self.dir)
            .and_then(|_| std::fs::write(&tmp, value))
            .and_then(|_| std::fs::rename(&tmp, &path));
        let _ = std::fs::remove_file(&tmp);
    }
}

/// Keeps nothing between runs: every run starts from the checkpoint.
#[derive(Clone, Debug, Default)]
pub struct NoStorage;

impl Storage for NoStorage {
    fn get(&self, _key: &str) -> Option<String> {
        None
    }
    fn set(&self, _key: &str, _value: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_the_npm_clients_file_names() {
        let s = FileStorage::new("/x");
        assert_eq!(
            s.file(LIGHT_CLIENT_KEY),
            PathBuf::from("/x/light-client.json")
        );
        assert_eq!(
            s.file(&format!("{LIGHT_CLIENT_KEY}:sepolia")),
            PathBuf::from("/x/light-client_sepolia.json")
        );
    }

    #[test]
    fn a_value_round_trips() {
        let dir = std::env::temp_dir().join(format!("willow-rpc-storage-{}", std::process::id()));
        let s = FileStorage::new(&dir);
        assert_eq!(s.get("k"), None);
        s.set("k", "v1");
        s.set("k", "v2");
        assert_eq!(s.get("k").as_deref(), Some("v2"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
