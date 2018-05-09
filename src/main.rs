#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate bspline;
extern crate timer;
extern crate chrono;

extern crate cgmath;

extern crate rosc;
use rosc::OscPacket;

extern crate rodio;
use rodio::Source;

use std::time::Duration;
use std::io::BufReader;
use std::env;
use std::net::{UdpSocket, SocketAddrV4};
use std::str::FromStr;
use std::sync::mpsc;

use std::collections::BinaryHeap;
use std::fs::File;
// use std::io::prelude::*;
use std::thread;

mod config;
use config::open_scene;
mod soundscape;
mod rodiox;

#[derive(Debug, Copy, Clone)]
enum OscEvent {
    Volume(f32),
    NoAction,
}

#[derive(Debug, Copy, Clone)]
enum AppMsg {
    Osc(OscEvent),
    MetroTick,
    Error
}

fn main() {
    // Handle args
    let args: Vec<String> = env::args().collect();
    let usage = format!("Usage {} [soundscape-config.yml]", &args[0]);

    let config_file_name =
        if args.len() > 2 {
            // Too many arguments
            println!("{}", usage);
            ::std::process::exit(1)
        }
        else if args.len() == 2 {
            // custom configuration
            format!("{}", args[1])
        }
        else {
            // Default value
            String::from("soundscape-config.yml")
        };


    // Read config
    let config = config::load_from_file(&config_file_name).expect("Unable to continue without valid configuration");
    println!("config: {:?}", config);

    // try bulding listening address
    let address = match SocketAddrV4::from_str( format!("{}:{}", config.listen_addr.host, config.listen_addr.port).as_str() ) {
        Ok(addr)    => addr,
        Err(_)      => {
            println!("Unable to use host and port config fields ('{}:{}') as an address!", config.listen_addr.host, config.listen_addr.port);
            ::std::process::exit(1)
        }
    };

    // test scene files
    for scene_file in &config.scenes {
        print!("Checking scene file: '{}'...", scene_file);
        let scene = open_scene(scene_file);
        for resource in &scene.resources {
            File::open(&resource.path)
                .expect( &format!("Error opening content for resource '{}' from scene '{}'", resource.path, scene_file) );
        }
        println!("Scene OK", );
    }

    let background_scene = match config.background_scene {
        Some (scene_file) => {
            let clone_path = scene_file.as_str().to_string();
            let scene = open_scene(&clone_path);
            for resource in &scene.resources {
                File::open(&resource.path)
                    .expect( &format!("Error opening content for resource '{}' from background scene '{}'", resource.path, scene_file) );
            }
            Some (scene)
        },
        None => {
            println!("No background scene specified");
            None
        },
    };

    // setup metronome
    let (tx_app_msg, rx_app_msg) = mpsc::channel();
    let tx_metro = mpsc::Sender::clone(&tx_app_msg);
    let metro = timer::MessageTimer::new(tx_metro);

    let step_size_ms = config.metro_step_ms;
    let metro_rate = step_size_ms as i64;
    let _guard_metro = metro.schedule_repeating(chrono::Duration::milliseconds(metro_rate), AppMsg::MetroTick);

    // setup command BinaryHeap and queue first play command
    let mut future_commands = BinaryHeap::with_capacity(128);
    future_commands.push(soundscape::load_at(0, 0));
    future_commands.push(soundscape::load_background());

    // Setup socket
    let _listener = thread::spawn(move || {
        let socket = UdpSocket::bind(address).unwrap();
        println!("Listening on {}...", address);

        // Block while listening for OSC messages
        let mut packet_buffer = [0u8; rosc::decoder::MTU];

        loop {
            match socket.recv_from(&mut packet_buffer) {
                Ok((bytes, _remote_address)) => {
                    let action = route_osc(
                        rosc::decoder::decode(&packet_buffer[..bytes]).unwrap()
                    );
                    tx_app_msg.send(AppMsg::Osc(action)).unwrap();
                }
                Err(e) => {
                    // Log to console and quit the recv loop
                    println!("Error receiving from socket: {}", e);
                    // break;
                }
            }
        }
    });

    // Setup audio

    let output_device = rodio::default_output_device().expect("Error selecting audio output device");

    println!("Outputing audio to {}", output_device.name());
    let output_count = match output_device.default_output_format() {
        Ok (format) => {
            println!("Using default output format of {:?}", format);
            format.channels as usize
        },
        Err (e) => {
            println!("Error retriving channel count from audio device: {:?}", e);
            ::std::process::exit(1);
        },
    };

    let mut speaker_positions :Vec<[f32; 3]> = Vec::with_capacity(output_count);
    for i in 0..output_count {
        if i < config.speaker_positions.positions.len() {
            speaker_positions.push(config.speaker_positions.positions[i])
        }
        else {
            println!("No speaker position defined for output {}", i);
        }
    }

    if speaker_positions.len() < config.speaker_positions.positions.len() {
        println!("Ignored speaker psotions for channels greater than {}", output_count);
    }

    let mut background_sources: Vec<soundscape::SoundSource> = Vec::new();
    let mut active_sources: Vec<soundscape::SoundSource> = Vec::with_capacity(config.voice_limit);
    let mut retired_sources: Vec<soundscape::SoundSource> = Vec::with_capacity(config.voice_limit);

    let mut elapsed_ms = 0u64;
    let mut dynamic_curve = soundscape::structure_from_scene(&open_scene(&config.scenes[0]));
    // let volume_curve = config::to_b_spline(&config.structure);

    // let duration = config.structure_duration_ms as f32;
    // let step_t = volume_curve.knot_domain().1 / duration;
    // let mut step = 0f32
    let step_increment = metro_rate as f32;
    // Run loop
    loop {
        let message = match rx_app_msg.recv() {
            Ok(msg) => msg,
            Err (e) => {
                println!("Error receiving AppMsg: {}", e);
                AppMsg::Error
            }
        };
        match message {
            AppMsg::Error => (),
            AppMsg::Osc (action) => {
                match action {
                    _ => println!("No action defined for {:?}", action),
                }
            }
            AppMsg::MetroTick => {
                elapsed_ms += step_size_ms;

                dynamic_curve.step += step_increment;
                if dynamic_curve.step > dynamic_curve.duration {
                    dynamic_curve.step = 0f32;
                }

                // execute any commands that should be executed now or earlier
                while soundscape::is_cmd_now(future_commands.peek(), &elapsed_ms) {
                    match future_commands.pop() {
                        Some(future_cmd) => {
                            match future_cmd.command {
                                soundscape::Cmd::Play => {
                                    println!("Executing play command at step: {}", elapsed_ms);
                                    play(&mut active_sources)
                                },
                                soundscape::Cmd::Load (n) => {
                                    println!("Executing load command at step: {}", elapsed_ms);
                                    let scene = open_scene(&config.scenes[n]);
                                    add_resources(&mut active_sources, &output_device, &scene, &speaker_positions);
                                    dynamic_curve = soundscape::structure_from_scene(&scene);

                                    future_commands.push(soundscape::play_at(elapsed_ms + step_size_ms));
                                    future_commands.push(soundscape::retire_at(elapsed_ms + scene.duration_ms));
                                    future_commands.push(soundscape::load_at(
                                            (n + 1) % config.scenes.len(),
                                            elapsed_ms + scene.duration_ms + step_size_ms
                                            ));
                                },
                                soundscape::Cmd::LoadBackground => {
                                    match background_scene {
                                        Some (ref scene) => {
                                            add_resources(&mut background_sources, &output_device, &scene, &speaker_positions);
                                            play(&mut background_sources);
                                            set_volume(&mut background_sources, 0.9);
                                        },
                                        None => (),
                                    }
                                },
                                soundscape::Cmd::Retire => {
                                    println!("Executing retire command at step: {}", elapsed_ms);
                                    retire_resources(&mut active_sources, &mut retired_sources);
                                }
                            }
                        },
                        None => println!("Expected to unpack command but no command was present. Unexpected state relating to future commands. Continuing execution."),
                    }
                }

                let t = dynamic_curve.step_t * dynamic_curve.step;
                let volume = dynamic_curve.spline.point( t );
                for c in &mut active_sources {
                    soundscape::update(c); // execute volume fade steps

                    if c.max_threshold > volume && c.min_threshold < volume {
                        if c.is_live == false {
                            c.is_live = true;
                            let volume = config.default_level + c.gain;
                            let fade_steps = c.fade_in_steps;
                            soundscape::volume_fade(c, volume, fade_steps)
                        }
                    }
                    else {
                        if c.is_live == true {
                            c.is_live = false;
                            let fade_steps = c.fade_out_steps;
                            soundscape::volume_fade(c, 0.0, fade_steps)
                        }
                    }
                }

                // run fades and remove any retired sources which have finished their fade out.
                for s in &mut retired_sources {
                    soundscape::update(s);
                }
                retired_sources.retain(|s| s.volume_updates > 0);

                //if elapsed_ms % 1000 == 0 {
                //    println!("v: {}, t: {}, pending commands: {}", volume, t, future_commands.len());
                //}
            }
        }
    }

}

fn route_osc(packet: OscPacket) -> OscEvent {
    match packet {
        OscPacket::Message(message) => {
            if message.addr == "/volume" {
                match message.args {
                    Some(arguments) => {
                        println!("OSC arguments: {:?}", arguments);
                        match arguments.first() {
                            Some(first_arg) => {
                                match first_arg {
                                    &rosc::OscType::Float(volume) => OscEvent::Volume(volume),
                                    &rosc::OscType::Double(volume) => {
                                        let volume = volume as f32;
                                        OscEvent::Volume(volume)
                                    }
                                    _ => {
                                        println!("/volume expected a float, but received: {:?}", arguments);
                                        OscEvent::NoAction
                                    },
                                }
                            }
                            None => {
                                println!("No arguments in message. Expected float for /volume.");
                                OscEvent::NoAction
                            }
                        }
                    }
                    None => {
                        println!("No arguments in message. Expected float for /volume.");
                        OscEvent::NoAction
                    }
                }
            }
            else {
                println!("No routing implemented for messages. Received: {:?}", message);
                OscEvent::NoAction
            }
        }
        OscPacket::Bundle(bundle) => {
            println!("No routing implemented for bundles. Received: {:?}", bundle);
            OscEvent::NoAction
        }
    }
}

// active_sources actions

// Load sound sources from config objects
fn add_resources(active_sources: &mut Vec<soundscape::SoundSource>, output_device: &rodio::Device, scene: &config::Scene, speakers: &Vec<[f32; 3]>) {
    println!("Loading {}", scene.name);
    for res in &scene.resources {
        println!("Adding: {:?}", res);
        let mut sound_source = soundscape::resource_to_sound_source(res, &output_device, &speakers);
        let source =
            config::res_to_file(&res.path).and_then(|file| {
                match rodio::Decoder::new( BufReader::new(file) ) {
                    Ok (decoder) => Ok(decoder),
                    Err (e) => Err( format!("Error creating audio source for '{}': {}", res.path, e) ),
                }
            })
            .expect("Error reading audio resource")
            .buffered()
            .repeat_infinite();

        // pause until a play command is executed
        sound_source.channel.pause();
        sound_source.channel.set_volume(0.0);

        match res.reverb {
            Some (ref params) => sound_source.channel.append(source.reverb(Duration::from_millis(params.delay_ms), params.mix_t)),
            None => sound_source.channel.append(source),
        }
        active_sources.push(sound_source)
    }
}

// Move from active Vec to retired Vec and apply fade out to each channel
fn retire_resources(active_sources: &mut Vec<soundscape::SoundSource>, retired_sources: &mut Vec<soundscape::SoundSource>) {
    retired_sources.append(active_sources);
    for s in retired_sources {
        let fade_steps = s.fade_out_steps;
        soundscape::volume_fade(s, -0.0, fade_steps);
    }
}

fn set_volume(channels: &mut Vec<soundscape::SoundSource>, volume: f32) {
    for c in channels {
        c.channel.set_volume(volume)
    }
}

fn play(channels: &mut Vec<soundscape::SoundSource>) {
    for c in channels {
        c.channel.play()
    }
}
