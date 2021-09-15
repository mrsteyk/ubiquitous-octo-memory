use sha2::Digest;
use std::error::Error;

#[derive(Debug)]
pub enum BlacklistReason {
    CSS,
    TF2,
    CSGO,
    Custom(String),
}

#[derive(Debug)]
pub struct Blacklist {
    pub hashes: hashbrown::HashMap<String, Vec<String>>,
}

impl Blacklist {
    pub fn new(data: &[u8]) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            hashes: serde_json::from_slice(data)?,
        })
    }

    pub fn check(&self, data: &[u8]) -> Option<BlacklistReason> {
        let file_hash = {
            let mut hasher = sha2::Sha256::new();
            hasher.update(data);
            hex::encode(hasher.finalize())
        };
        // I feel like there's a better way to do this...
        for (name, hashes) in &self.hashes {
            for hash in hashes {
                if file_hash.eq(&hash.to_lowercase()) {
                    return Some(match name.to_lowercase().as_str() {
                        "css" => BlacklistReason::CSS,
                        "tf2" => BlacklistReason::TF2,
                        "csgo" => BlacklistReason::CSGO,
                        _ => BlacklistReason::Custom(name.clone()),
                    });
                }
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn check_image_data(&self, _data: &[u8]) -> Option<BlacklistReason> {
        todo!("Deprecate self.check and make this the default?")
    }
}
