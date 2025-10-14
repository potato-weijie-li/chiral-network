// Keystore functionality for node crate
// This module provides keystore operations without Tauri dependencies

use aes::cipher::{KeyIvInit, StreamCipher};
use aes::Aes256;
use ctr::Ctr128BE;
use directories::ProjectDirs;
use hmac::Hmac;
use pbkdf2::pbkdf2;
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};
use sha3::Sha3_256;
use std::fs;
use std::path::PathBuf;

type Aes256Ctr = Ctr128BE<Aes256>;

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedKeystore {
    pub address: String,
    pub encrypted_private_key: String,
    pub salt: String,
    pub iv: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_two_fa_secret: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub two_fa_iv: Option<String>,
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub file_encryption_keys: std::collections::HashMap<String, EncryptedFileKey>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedFileKey {
    pub encrypted_key: String,
    pub key_iv: String,
    pub file_hash: String,
    pub created_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Keystore {
    pub accounts: Vec<EncryptedKeystore>,
}

impl Keystore {
    pub fn new() -> Self {
        Keystore {
            accounts: Vec::new(),
        }
    }

    pub fn get_keystore_path() -> Result<PathBuf, String> {
        let proj_dirs = ProjectDirs::from("com", "chiral", "network")
            .ok_or_else(|| "Could not determine project directories".to_string())?;

        let data_dir = proj_dirs.data_dir();

        // Create directory if it doesn't exist
        fs::create_dir_all(data_dir)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;

        Ok(data_dir.join("keystore.json"))
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::get_keystore_path()?;

        if !path.exists() {
            return Ok(Self::new());
        }

        let contents =
            fs::read_to_string(&path).map_err(|e| format!("Failed to read keystore: {}", e))?;

        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse keystore: {}", e))
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_keystore_path()?;

        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize keystore: {}", e))?;

        fs::write(&path, contents).map_err(|e| format!("Failed to write keystore: {}", e))?;

        Ok(())
    }

    pub fn add_account(
        &mut self,
        address: String,
        private_key: &str,
        password: &str,
    ) -> Result<(), String> {
        let (encrypted, salt, iv) = encrypt_private_key(private_key, password)?;

        // Remove existing account with same address
        self.accounts.retain(|a| a.address != address);

        self.accounts.push(EncryptedKeystore {
            address,
            encrypted_private_key: encrypted,
            salt,
            iv,
            encrypted_two_fa_secret: None,
            two_fa_iv: None,
            file_encryption_keys: std::collections::HashMap::new(),
        });

        self.save()?;
        Ok(())
    }

    pub fn get_account(&self, address: &str, password: &str) -> Result<String, String> {
        let account = self
            .accounts
            .iter()
            .find(|a| a.address == address)
            .ok_or_else(|| "Account not found".to_string())?;

        decrypt_private_key(
            &account.encrypted_private_key,
            &account.salt,
            &account.iv,
            password,
        )
    }

    pub fn list_accounts(&self) -> Vec<String> {
        self.accounts.iter().map(|a| a.address.clone()).collect()
    }
}

fn encrypt_private_key(
    private_key: &str,
    password: &str,
) -> Result<(String, String, String), String> {
    let mut salt = [0u8; 32];
    thread_rng().fill_bytes(&mut salt);

    let mut key = [0u8; 32];
    pbkdf2::<Hmac<Sha3_256>>(password.as_bytes(), &salt, 10000, &mut key)
        .map_err(|e| format!("Key derivation failed: {}", e))?;

    let mut iv = [0u8; 16];
    thread_rng().fill_bytes(&mut iv);

    let mut cipher = Aes256Ctr::new(&key.into(), &iv.into());
    let mut data = private_key.as_bytes().to_vec();
    cipher.apply_keystream(&mut data);

    Ok((hex::encode(data), hex::encode(salt), hex::encode(iv)))
}

fn decrypt_private_key(
    encrypted: &str,
    salt: &str,
    iv: &str,
    password: &str,
) -> Result<String, String> {
    let encrypted_bytes = hex::decode(encrypted).map_err(|e| format!("Invalid hex: {}", e))?;
    let salt_bytes = hex::decode(salt).map_err(|e| format!("Invalid salt: {}", e))?;
    let iv_bytes = hex::decode(iv).map_err(|e| format!("Invalid IV: {}", e))?;

    let mut key = [0u8; 32];
    pbkdf2::<Hmac<Sha3_256>>(password.as_bytes(), &salt_bytes, 10000, &mut key)
        .map_err(|e| format!("Key derivation failed: {}", e))?;

    let mut cipher = Aes256Ctr::new(&key.into(), &iv_bytes.as_slice().try_into().unwrap());
    let mut data = encrypted_bytes;
    cipher.apply_keystream(&mut data);

    String::from_utf8(data).map_err(|e| format!("Invalid UTF-8: {}", e))
}

// Public API functions that match the Tauri command signatures

/// Save an account to the keystore
pub async fn save_account_to_keystore(
    address: String,
    private_key: String,
    password: String,
) -> Result<(), String> {
    let mut keystore = Keystore::load()?;
    keystore.add_account(address, &private_key, &password)?;
    Ok(())
}

/// Load an account from the keystore
pub async fn load_account_from_keystore(
    address: String,
    password: String,
) -> Result<String, String> {
    let keystore = Keystore::load()?;
    keystore.get_account(&address, &password)
}

/// List all accounts in the keystore
pub async fn list_keystore_accounts() -> Result<Vec<String>, String> {
    let keystore = Keystore::load()?;
    Ok(keystore.list_accounts())
}
