use std::fs::File;
use std::io::prelude::*;

use serde_yaml;
use bspline;

// Configuration structs

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct BSplineParams {
    pub points: Vec<f32>,
    pub knots:  Vec<f32>,
    pub degree: usize,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundResource {
    pub path:           String,
    pub min_threshold:  f32,
    pub max_threshold:  f32,
    pub gain:           f32,
    pub fade_in_steps:  Option<u32>,
    pub fade_out_steps: Option<u32>,
    pub reverb:         Option<ReverbParams>,
    pub position:       Option<[f32; 3]>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ReverbParams {
    pub delay_ms:   u64,
    pub mix_t:      f32,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub name:               String,
    pub duration_ms:        u64,
    pub cycle_duration_ms:  u64,
    pub resources:          Vec<SoundResource>,
    pub structure:          BSplineParams,
}

pub fn open_scene(file: &String) -> Scene {
    let mut scene_file = File::open(file)
        .expect( &format!("Error opening file '{}'", file) );

    let mut scene_contents = String::new();
    scene_file.read_to_string(&mut scene_contents)
        .expect( &format!("Error reading scene file '{}'", file) );

    serde_yaml::from_str(&scene_contents)
        .expect( &format!("Error parsing scene file '{}'", file) )
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Address {
    pub host:   String,
    pub port:  u32,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Speakers {
    pub positions:   Vec<[f32; 3]>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Soundscape {
    pub listen_addr:            Address,
    pub subscribers:            Vec<Address>,
    pub scenes:                 Vec<String>,
    pub metro_step_ms:          u64,
    // pub structure_duration_ms:  usize,
    pub voice_limit:            usize,
    pub default_level:          f32,
    pub background_scene:       Option<String>,
    pub speaker_positions:      Speakers,
    pub ignore_extra_speakers:  Option<bool>,
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

pub fn to_b_spline(params: &BSplineParams) -> bspline::BSpline<f32> {
    let points = params.points.to_owned();
    let knots = params.knots.to_owned();
    bspline::BSpline::new(params.degree, points, knots)
}

pub fn res_to_file(resource: &String) -> Result<File, String> {
    match File::open(resource) {
        Ok (file) => Ok(file),
        Err (e) => return Err( format!("Error opening audio file '{}': {}", resource, e) ),
    }
}
