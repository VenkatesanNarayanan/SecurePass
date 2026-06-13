use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use chacha20poly1305::Nonce;
use crate::crypto::{CryptoManager, generate_aad};

/// Represents a password entry in the vault
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entry {
    pub id: Uuid,
    pub category: String,
    pub title: String,
    pub fields: Vec<Field>,
    pub tags: Vec<String>,
    pub notes: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entry {
    /// Create a new entry
    pub fn new(category: String, title: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            category,
            title,
            fields: Vec::new(),
            tags: Vec::new(),
            notes: String::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the entry's timestamp
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Add a field to the entry
    pub fn add_field(&mut self, name: String, field_type: FieldType, value: String) {
        let field = Field::new(name, field_type, value);
        self.fields.push(field);
        self.touch();
    }

    /// Get field value
    #[allow(dead_code)]
    pub fn get_field_value(&self, field_name: &str, _crypto: &CryptoManager) -> Result<Option<String>, Box<dyn std::error::Error>> {
        for field in &self.fields {
            if field.name == field_name {
                return field.get_plaintext_value().map(Some);
            }
        }
        Ok(None)
    }

    /// Update field value
    #[allow(dead_code)]
    pub fn update_field(&mut self, field_name: &str, new_value: String, _crypto: &CryptoManager) -> Result<(), Box<dyn std::error::Error>> {
        for field in &mut self.fields {
            if field.name == field_name {
                field.value_encrypted = new_value.into_bytes(); // Temporary implementation
                self.touch();
                return Ok(());
            }
        }
        Err(format!("Field '{}' not found", field_name).into())
    }
}

/// Represents a field within an entry
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Field {
    pub name: String,
    pub field_type: FieldType,
    pub value_encrypted: Vec<u8>,
    pub nonce: Vec<u8>,
}

impl Field {
    /// Create a new field (plaintext - will be encrypted when saved)
    pub fn new(name: String, field_type: FieldType, plaintext_value: String) -> Self {
        Self {
            name,
            field_type,
            value_encrypted: plaintext_value.into_bytes(), // Temporary - will be encrypted
            nonce: vec![0; 12], // Temporary - will be proper nonce
        }
    }

    /// Encrypt the field value
    #[allow(dead_code)]
    pub fn encrypt_value(&mut self, crypto: &CryptoManager) -> Result<(), Box<dyn std::error::Error>> {
        let nonce = CryptoManager::generate_nonce();
        let aad = generate_aad(&self.name, Utc::now().timestamp(), "field");

        let plaintext = String::from_utf8(self.value_encrypted.clone())
            .map_err(|e| format!("Invalid UTF-8 in field value: {}", e))?;

        self.value_encrypted = crypto.encrypt(plaintext.as_bytes(), &nonce, &aad)?;
        self.nonce = nonce.to_vec();
        Ok(())
    }

    /// Decrypt the field value
    pub fn decrypt_value(&self, crypto: &CryptoManager) -> Result<String, Box<dyn std::error::Error>> {
        let nonce = Nonce::from_slice(&self.nonce);
        let aad = generate_aad(&self.name, Utc::now().timestamp(), "field");

        let decrypted_bytes = crypto.decrypt(&self.value_encrypted, nonce, &aad)?;
        String::from_utf8(decrypted_bytes)
            .map_err(|e| format!("Invalid UTF-8 in decrypted field: {}", e).into())
    }

    /// Get the plaintext value (assumes it's already plaintext)
    pub fn get_plaintext_value(&self) -> Result<String, Box<dyn std::error::Error>> {
        String::from_utf8(self.value_encrypted.clone())
            .map_err(|e| format!("Invalid UTF-8 in field value: {}", e).into())
    }
}

/// Field types supported by SecurePass
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum FieldType {
    Text,
    Password,
    LongText,
}

/// Tombstone left when an entry is deleted, used by the sync merge algorithm.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeletedEntry {
    pub id: Uuid,
    pub deleted_at: DateTime<Utc>,
}

/// The main vault containing all entries
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Vault {
    pub entries: Vec<Entry>,
    pub version: String,
    pub salt: [u8; 32],
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Tombstones for deleted entries — allows sync to propagate deletions.
    #[serde(default)]
    pub deleted_entries: Vec<DeletedEntry>,
}

impl Vault {
    /// Create a new empty vault
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            entries: Vec::new(),
            version: "1.0".to_string(),
            salt: CryptoManager::generate_salt(),
            created_at: now,
            updated_at: now,
            deleted_entries: Vec::new(),
        }
    }

    /// Add an entry to the vault
    pub fn add_entry(&mut self, entry: Entry) {
        self.entries.push(entry);
        self.updated_at = Utc::now();
    }

    /// Remove an entry from the vault and record a tombstone for sync.
    pub fn remove_entry(&mut self, entry_id: &Uuid) -> bool {
        let initial_len = self.entries.len();
        self.entries.retain(|e| e.id != *entry_id);
        let removed = self.entries.len() < initial_len;
        if removed {
            self.deleted_entries.push(DeletedEntry { id: *entry_id, deleted_at: Utc::now() });
            self.updated_at = Utc::now();
        }
        removed
    }

    /// Get an entry by ID
    pub fn get_entry(&self, entry_id: &Uuid) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == *entry_id)
    }

    /// Get a mutable reference to an entry by ID
    pub fn get_entry_mut(&mut self, entry_id: &Uuid) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|e| e.id == *entry_id)
    }

    /// Search entries by title, username, or tags
    pub fn search(&self, query: &str) -> Vec<&Entry> {
        let query_lower = query.to_lowercase();
        self.entries.iter().filter(|entry| {
            entry.title.to_lowercase().contains(&query_lower) ||
            entry.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower)) ||
            entry.fields.iter().any(|field| {
                // Note: In a real implementation, we'd decrypt and search
                // For now, we'll skip field content search for security
                field.name.to_lowercase().contains(&query_lower)
            })
        }).collect()
    }

    /// Filter entries by category
    #[allow(dead_code)]
    pub fn filter_by_category(&self, category: &str) -> Vec<&Entry> {
        self.entries.iter().filter(|entry| entry.category == category).collect()
    }

    /// Get all unique categories
    pub fn get_categories(&self) -> Vec<String> {
        let mut categories: Vec<String> = self.entries.iter()
            .map(|e| e.category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        categories.sort();
        categories
    }

    /// Get all unique tags
    pub fn get_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self.entries.iter()
            .flat_map(|e| e.tags.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        tags.sort();
        tags
    }
}

/// Vault metadata for serialization
#[derive(Serialize, Deserialize)]
pub struct VaultMetadata {
    pub version: String,
    pub entry_count: usize,
    pub categories: Vec<String>,
    pub last_updated: DateTime<Utc>,
}

impl From<&Vault> for VaultMetadata {
    fn from(vault: &Vault) -> Self {
        Self {
            version: vault.version.clone(),
            entry_count: vault.entries.len(),
            categories: vault.get_categories(),
            last_updated: vault.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_creation() {
        let entry = Entry::new("web-app".to_string(), "Google".to_string());
        assert_eq!(entry.category, "web-app");
        assert_eq!(entry.title, "Google");
        assert!(entry.fields.is_empty());
    }

    #[test]
    fn test_vault_operations() {
        let mut vault = Vault::new();
        let entry = Entry::new("web-app".to_string(), "Test".to_string());

        let entry_id = entry.id;
        vault.add_entry(entry);

        assert_eq!(vault.entries.len(), 1);
        assert!(vault.get_entry(&entry_id).is_some());

        vault.remove_entry(&entry_id);
        assert_eq!(vault.entries.len(), 0);
    }

    #[test]
    fn test_vault_search() {
        let mut vault = Vault::new();

        let mut entry1 = Entry::new("web-app".to_string(), "Google".to_string());
        entry1.tags = vec!["work".to_string()];
        vault.add_entry(entry1);

        let mut entry2 = Entry::new("ssh-server".to_string(), "Server".to_string());
        entry2.tags = vec!["personal".to_string()];
        vault.add_entry(entry2);

        let results = vault.search("Google");
        assert_eq!(results.len(), 1);

        let results = vault.search("work");
        assert_eq!(results.len(), 1);
    }
}