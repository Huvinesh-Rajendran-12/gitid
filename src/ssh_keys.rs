use anyhow::{Context, Result, bail};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Represents a discovered SSH key pair
#[derive(Debug, Clone)]
pub struct SshKey {
    pub name: String,
    pub private_key_path: PathBuf,
    pub public_key_path: PathBuf,
    pub key_type: String,
}

impl SshKey {
    /// Get the path as a string with ~ for home directory
    pub fn path_display(&self) -> String {
        if let Some(home) = dirs::home_dir() {
            if let Ok(relative) = self.private_key_path.strip_prefix(&home) {
                return format!("~/{}", relative.display());
            }
        }
        self.private_key_path.display().to_string()
    }
}

/// Get the SSH directory path
pub fn ssh_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".ssh"))
}

/// Discover existing SSH keys in ~/.ssh/
pub fn discover_keys() -> Result<Vec<SshKey>> {
    let ssh_path = ssh_dir()?;

    if !ssh_path.exists() {
        return Ok(Vec::new());
    }

    let mut keys = Vec::new();

    let entries = fs::read_dir(&ssh_path)
        .with_context(|| format!("Failed to read SSH directory: {}", ssh_path.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Look for private keys (files without .pub extension that have a matching .pub file)
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            // Skip public keys, config, known_hosts, etc.
            if filename.ends_with(".pub")
                || filename == "config"
                || filename == "known_hosts"
                || filename == "known_hosts.old"
                || filename == "authorized_keys"
                || filename.starts_with(".")
            {
                continue;
            }

            // Check if corresponding .pub file exists
            let pub_path = PathBuf::from(format!("{}.pub", path.display()));

            if pub_path.exists() {
                let key_type = detect_key_type(&path);
                keys.push(SshKey {
                    name: filename.to_string(),
                    private_key_path: path,
                    public_key_path: pub_path,
                    key_type,
                });
            }
        }
    }

    // Sort by name
    keys.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(keys)
}

/// Detect the type of SSH key (ed25519, rsa, ecdsa, etc.)
fn detect_key_type(path: &PathBuf) -> String {
    // Try to read the first line of the private key to determine type
    if let Ok(content) = fs::read_to_string(path) {
        let first_line = content.lines().next().unwrap_or("");
        if first_line.contains("OPENSSH PRIVATE KEY") {
            // Modern OpenSSH format - need to check the public key or filename
            let filename = path.file_name().unwrap_or_default().to_str().unwrap_or("");
            if filename.contains("ed25519") {
                return "ed25519".to_string();
            } else if filename.contains("ecdsa") {
                return "ecdsa".to_string();
            } else if filename.contains("rsa") {
                return "rsa".to_string();
            }
            // Try to detect from public key
            let pub_path = PathBuf::from(format!("{}.pub", path.display()));
            if let Ok(pub_content) = fs::read_to_string(&pub_path) {
                if pub_content.starts_with("ssh-ed25519") {
                    return "ed25519".to_string();
                } else if pub_content.starts_with("ssh-rsa") {
                    return "rsa".to_string();
                } else if pub_content.starts_with("ecdsa-") {
                    return "ecdsa".to_string();
                }
            }
            return "openssh".to_string();
        } else if first_line.contains("RSA PRIVATE KEY") {
            return "rsa".to_string();
        } else if first_line.contains("EC PRIVATE KEY") {
            return "ecdsa".to_string();
        }
    }
    "unknown".to_string()
}

/// Generate a new SSH key pair
pub fn generate_key(name: &str, email: &str) -> Result<SshKey> {
    let ssh_path = ssh_dir()?;

    // Ensure .ssh directory exists with correct permissions
    if !ssh_path.exists() {
        fs::create_dir_all(&ssh_path)
            .with_context(|| format!("Failed to create SSH directory: {}", ssh_path.display()))?;

        // Set directory permissions to 700 on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&ssh_path, fs::Permissions::from_mode(0o700))?;
        }
    }

    let key_filename = format!("id_ed25519_{}", name);
    let private_key_path = ssh_path.join(&key_filename);
    let public_key_path = ssh_path.join(format!("{}.pub", key_filename));

    // Check if key already exists
    if private_key_path.exists() {
        bail!("SSH key already exists: {}", private_key_path.display());
    }

    // Generate the key using ssh-keygen
    let status = Command::new("ssh-keygen")
        .args([
            "-t", "ed25519",
            "-C", email,
            "-f", private_key_path.to_str().unwrap(),
            "-N", "",  // Empty passphrase (user can change later)
        ])
        .status()
        .context("Failed to run ssh-keygen. Is OpenSSH installed?")?;

    if !status.success() {
        bail!("ssh-keygen failed to generate key");
    }

    Ok(SshKey {
        name: key_filename,
        private_key_path,
        public_key_path,
        key_type: "ed25519".to_string(),
    })
}

/// Get the public key content (for display/copying)
pub fn read_public_key(key: &SshKey) -> Result<String> {
    fs::read_to_string(&key.public_key_path)
        .with_context(|| format!("Failed to read public key: {}", key.public_key_path.display()))
}
