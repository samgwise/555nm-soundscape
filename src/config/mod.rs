use std::fs::File;
use std::io::prelude::*;

use serde_yaml;

// Configuration structs
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundResource {
    pub path: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Soundscape {
    pub resources:  Vec<SoundResource>,
    pub host:       String,
    pub port:       u32,
}

pub fn load_from_file(file_name: &String) -> Result<Soundscape, String> {
    let mut config_file = match File::open(file_name) {
        Ok (f) => f,
        Err (e) => return Err( format!("Error opening file '{}': {}", file_name, e) ),
    };

    let mut config_contents = String::new();
    match config_file.read_to_string(&mut config_contents) {
        Ok (_) => {
            match serde_yaml::from_str(&config_contents) {
                Ok (config) => Ok(config),
                Err (e) => Err( format!("Error parsing config from '{}': {}", file_name, e) ),
            }
        },
        Err (e) => Err( format!("Error reading from file '{}': {}", file_name, e) ),
    }
}
