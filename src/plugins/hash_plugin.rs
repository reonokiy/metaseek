//! Hash plugin for generating hashes

use super::traits::{Plugin, PluginInfo};
use crate::results::Answer;
use sha2::{Digest, Sha256, Sha512};

/// Plugin for generating cryptographic hashes
pub struct HashPlugin;

impl HashPlugin {
    pub fn new() -> Self {
        Self
    }

    fn compute_hash(&self, algorithm: &str, input: &str) -> Option<String> {
        match algorithm.to_lowercase().as_str() {
            "md5" => {
                let digest = md5::compute(input.as_bytes());
                Some(format!("{:x}", digest))
            }
            "sha256" | "sha-256" => {
                let mut hasher = Sha256::new();
                hasher.update(input.as_bytes());
                Some(hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect::<String>())
            }
            "sha512" | "sha-512" => {
                let mut hasher = Sha512::new();
                hasher.update(input.as_bytes());
                Some(hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect::<String>())
            }
            _ => None,
        }
    }
}

impl Default for HashPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for HashPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "hash_plugin".to_string(),
            name: "Hash Generator".to_string(),
            description: "Generate MD5, SHA-256, SHA-512 hashes".to_string(),
            default_on: true,
        }
    }

    fn keywords(&self) -> Vec<&str> {
        vec!["md5", "sha256", "sha512", "sha-256", "sha-512", "hash"]
    }

    fn process(&self, query: &str) -> Option<Answer> {
        let query = query.trim().to_lowercase();

        // Parse query: "md5 hello" or "sha256 test"
        let parts: Vec<&str> = query.splitn(2, ' ').collect();
        if parts.len() != 2 {
            return None;
        }

        let algorithm = parts[0];
        let input = parts[1].trim();

        if input.is_empty() {
            return None;
        }

        self.compute_hash(algorithm, input).map(|hash| {
            Answer::new(
                format!(
                    "{} hash of \"{}\": {}",
                    algorithm.to_uppercase(),
                    input,
                    hash
                ),
                "hash_plugin".to_string(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5_hash() {
        let plugin = HashPlugin::new();
        let result = plugin.process("md5 hello");
        assert!(result.is_some());
        let answer = result.unwrap();
        assert!(answer.answer.contains("5d41402abc4b2a76b9719d911017c592"));
    }

    #[test]
    fn test_sha256_hash() {
        let plugin = HashPlugin::new();
        let result = plugin.process("sha256 hello");
        assert!(result.is_some());
    }
}
