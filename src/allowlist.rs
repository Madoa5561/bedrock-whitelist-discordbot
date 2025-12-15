use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllowlistEntry {
    pub name: String,
    pub ignores_player_limit: bool,
}

pub struct AllowlistManager {
    path: String,
}

impl AllowlistManager {
    pub fn new(path: String) -> Self {
        Self { path }
    }

    /// Read the allowlist from file
    pub async fn read(&self) -> Result<Vec<AllowlistEntry>, Box<dyn std::error::Error + Send + Sync>> {
        let content = fs::read_to_string(&self.path).await?;
        let entries: Vec<AllowlistEntry> = serde_json::from_str(&content)?;
        Ok(entries)
    }

    /// Add a new entry to the allowlist (checks for duplicates)
    pub async fn add_entry(&self, name: String) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let mut entries = self.read().await?;
        
        // Check if already exists
        if entries.iter().any(|e| e.name == name) {
            return Ok(false); // Already exists
        }

        // Add new entry
        entries.push(AllowlistEntry {
            name,
            ignores_player_limit: false,
        });

        // Write back to file atomically
        self.write(&entries).await?;
        Ok(true)
    }

    /// Write the allowlist to file
    async fn write(&self, entries: &[AllowlistEntry]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let json = serde_json::to_string_pretty(entries)?;
        
        // Write to a temporary file first, then rename (atomic operation)
        let temp_path = format!("{}.tmp", self.path);
        let mut file = fs::File::create(&temp_path).await?;
        file.write_all(json.as_bytes()).await?;
        file.sync_all().await?;
        drop(file); // Explicitly close the file
        
        // Rename is atomic on most filesystems
        fs::rename(&temp_path, &self.path).await?;
        
        Ok(())
    }
}
