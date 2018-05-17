use std::fs::File;
use std::io::prelude::*;
use chrono::prelude::*;
use chrono::Duration;
use ::epochsy;

use serde_yaml;
use bspline;

mod config_tests;
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

pub fn check_scene_file(scene_file :&String) -> Result<Scene, &'static str> {
    let scene = open_scene(&scene_file);
    for resource in &scene.resources {
        File::open(&resource.path)
            .expect( &format!("Error opening content for resource '{}' from background scene '{}'", resource.path, scene_file) );
    }
    Ok(scene)
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
pub struct DailySchedule {
    pub start:  String,
    pub end:    String,
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
    pub is_fallback_slave:      Option<bool>,
    pub daily_schedule:         Option<DailySchedule>,
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

pub fn is_fallback_slave(config :&Soundscape) -> bool {
    match config.is_fallback_slave {
        Some (is_slave) => is_slave,
        None            => false,
    }
}

pub fn ignore_extra_speakers(config :&Soundscape) -> bool {
    match config.ignore_extra_speakers {
        Some (is_ignored)   => is_ignored,
        None                => false,
    }
}

// convert a NaiveTime to the next epoch seconds occourence
pub fn next_epoch(clock_time :&epochsy::DateTime, from :&epochsy::DateTime) -> epochsy::DateTime {
    let cur_days = epochsy::floor_to_days(from);
    let from_clock_time = epochsy::sub(from, &cur_days);
    // force later to time to be later than from
    let to = match from_clock_time.moment <= clock_time.moment {
        true => epochsy::sum(&cur_days, clock_time),
        false => epochsy::sum(&epochsy::days_later(&cur_days, 1), clock_time)
    };
    // Add on the difference between from and to the from DateTime
    epochsy::add(
        from,
        &epochsy::diff(
            from,
            &to
        )
    )
}

// returns the next epoch in seconds when we should start
pub fn next_start_time(config: &Soundscape, from: &epochsy::DateTime) -> epochsy::DateTime {
    match config.daily_schedule {
        Some (ref schedule) => {
            let time = NaiveTime::parse_from_str(schedule.start.as_str(), "%H:%M:%S")
                .expect("Unable to use provided time.");
            next_epoch(
                &epochsy::hms(time.hour() as u64, time.minute() as u64, time.second() as u64)
                , from
            )
        },
        None => epochsy::hms(0, 0, 0),
    }
}

// Returns the next end time of the provided schedule, returns None if no schedule is defined in config
pub fn next_end_time(config: &Soundscape, from: &epochsy::DateTime) -> Option<epochsy::DateTime> {
    match config.daily_schedule {
        Some ( ref schedule) => {
            let time = NaiveTime::parse_from_str(schedule.end.as_str(), "%H:%M:%S")
                .expect("Unable to use provided time.");
            Some (
                next_epoch(
                    &epochsy::hms(time.hour() as u64, time.minute() as u64, time.second() as u64)
                    , from
                )
            )
        },
        None => None
    }
}

// // convert two NaiveTime objects into a forward counting millisecond value
// // accurate to seconds
// pub fn forward_duration_ms(start: &NaiveTime, end: &NaiveTime) -> i64 {
//     forward_duration_sec(start, end) * 1000
// }
//
// // convert two NaiveTime objects into a forward counting seconds value
// // accurate to seconds
// pub fn forward_duration_sec(start: &epochsy::DateTime, end: &NaiveTime) -> epochsy::Interval {
//     epochsy::diff(start,
//         &epochsy::hours_later(
//             &epochsy::minutes_later(
//                 &epochsy::seconds_later(
//                     start,
//                     end
//                 )
//             )
//         )
//     )
// }

pub fn is_in_schedule(now :&epochsy::DateTime, start: &epochsy::DateTime, end :&epochsy::DateTime) -> bool {
    if start.moment <= now.moment && now.moment <= end.moment {
        true
    }
    else {
        false
    }
}

pub fn now() -> DateTime<Local> {
    Local::now()
}

pub fn from_timestamp(instant: i64) -> DateTime<Local> {
    Local.timestamp(instant, 0)
}
