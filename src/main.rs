#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate bspline;
extern crate timer;
extern crate chrono;

extern crate rosc;
use rosc::OscPacket;

extern crate rodio;
use rodio::Source;

use std::time::Duration;
use std::io::BufReader;
use std::env;
use std::net::{UdpSocket, SocketAddrV4};
use std::str::FromStr;
use std::sync::mpsc::channel;
use std::collections::BinaryHeap;

mod config;
mod soundscape;

enum OscEvent {
    Volume(f32),
    NoAction,
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
            // custom configuration file
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
    let address = match SocketAddrV4::from_str( format!("{}:{}", config.host, config.port).as_str() ) {
        Ok(addr)    => addr,
        Err(_)      => {
            println!("Unable to use host and port config fields ('{}:{}') as an address!", config.host, config.port);
            ::std::process::exit(1)
        }
    };

    // setup metronome
    let (tx_metro, rx_metro) = channel();
    let metro = timer::MessageTimer::new(tx_metro);

    let step_size_ms = config.metro_step_ms;
    let metro_rate = step_size_ms as i64;
    let _guard_metro = metro.schedule_repeating(chrono::Duration::milliseconds(metro_rate), 0);

    // setup command BinaryHeap and queue first play command
    let mut future_commands = BinaryHeap::with_capacity(128);
    future_commands.push(soundscape::load_at(0, 0));

    // Setup audio

    let endpoint = rodio::default_endpoint().expect("Error selecting audio output device");

    let mut active_sources: Vec<soundscape::SoundSource> = Vec::with_capacity(config.voice_limit);
    let mut retired_sources: Vec<soundscape::SoundSource> = Vec::with_capacity(config.voice_limit);

    let mut elapsed_ms = 0u64;
    let mut dynamic_curve = soundscape::structure_from_scene(&config.scenes[0]);
    // let volume_curve = config::to_b_spline(&config.structure);
    //
    // let duration = config.structure_duration_ms as f32;
    // let step_t = volume_curve.knot_domain().1 / duration;
    // let mut step = 0f32;
    let step_increment = metro_rate as f32;
    // Run loop
    loop {
        let _ = rx_metro.recv();
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
                            add_resources(&mut active_sources, &endpoint, &config.scenes[n]);
                            dynamic_curve = soundscape::structure_from_scene(&config.scenes[n]);

                            future_commands.push(soundscape::play_at(elapsed_ms + step_size_ms));
                            future_commands.push(soundscape::retire_at(elapsed_ms + config.scenes[n].duration_ms));
                            future_commands.push(soundscape::load_at(
                                (n + 1) % config.scenes.len(),
                                elapsed_ms + config.scenes[n].duration_ms + step_size_ms
                            ));
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
                    let volume = 1.0 + c.gain;
                    soundscape::volume_fade(c, volume, 100)
                }
            }
            else {
                if c.is_live == true {
                    c.is_live = false;
                    soundscape::volume_fade(c, 0.0, 100)
                }
            }
        }

        // run fades and remove any retired sources which have finished their fade out.
        for s in &mut retired_sources {
            soundscape::update(s);
        }
        retired_sources.retain(|s| s.volume_updates > 0);

        if elapsed_ms % 1000 == 0 {
            println!("v: {}, t: {}, pending commands: {}", volume, t, future_commands.len());
        }
    }

    // // Setup socket
    // let socket = UdpSocket::bind(address).unwrap();
    // println!("Listening on {}...", address);
    //
    // // Block while listening for OSC messages
    // let mut packet_buffer = [0u8; rosc::decoder::MTU];
    //
    // loop {
    //     match socket.recv_from(&mut packet_buffer) {
    //         Ok((bytes, _remote_address)) => {
    //             let routing = route_osc(
    //                 rosc::decoder::decode(&packet_buffer[..bytes]).unwrap()
    //             );
    //
    //             match routing {
    //                 OscEvent::Volume(volume) => set_volume(&mut active_sources, volume),
    //                 OscEvent::NoAction       => println!("No action defined."),
    //             }
    //         }
    //         Err(e) => {
    //             // Log to console and quit the recv loop
    //             println!("Error receiving from socket: {}", e);
    //             break;
    //         }
    //     }
    // }
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
fn add_resources(active_sources: &mut Vec<soundscape::SoundSource>, endpoint: &rodio::Endpoint, scene: &config::Scene) {
    println!("Loading {}", scene.name);
    for res in &scene.resources {
        println!("Adding: {:?}", res);
        let mut sound_source = soundscape::resource_to_sound_source(res, &endpoint);
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
        soundscape::volume_fade(s, -0.0, 100);
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
