#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate bspline;
extern crate timer;
extern crate chrono;

extern crate cgmath;

extern crate rosc;
use rosc::OscPacket;
use rosc::OscMessage;

extern crate rodio;
use rodio::Source;

use std::time::Duration;
use std::io::BufReader;
use std::env;
use std::net::{UdpSocket, SocketAddrV4};
use std::str::FromStr;
use std::sync::mpsc;

use std::collections::BinaryHeap;
use std::thread;

mod config;
mod epochsy;
use config::open_scene;
mod soundscape;
mod rodiox;

#[derive(Debug, Copy, Clone)]
enum OscEvent {
    Volume(f32),
    MasterAlive(i64),
    SceneChange(usize, i64),
    RefreshBackground,
    NoAction,
}

#[derive(Debug, Copy, Clone)]
enum AppMsg {
    Osc(OscEvent),
    MetroTick,
    Update,
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

    // try bulding listening address (clone values too while we're here)
    let address = match SocketAddrV4::from_str( format!("{}:{}", config.listen_addr.host.as_str().to_string(), config.listen_addr.port).as_str() ) {
        Ok(addr)    => addr,
        Err(_)      => {
            println!("Unable to use host and port config fields ('{}:{}') as an address!", config.listen_addr.host, config.listen_addr.port);
            ::std::process::exit(1)
        }
    };

    let osc_out_addr = match SocketAddrV4::from_str( format!("{}:{}", config.listen_addr.host.as_str().to_string(), config.listen_addr.port + 1).as_str() ) {
        Ok(addr)    => addr,
        Err(_)      => {
            println!("Unable to use host and port config fields ('{}:{}') as an address!", config.listen_addr.host, config.listen_addr.port + 1);
            ::std::process::exit(1)
        }
    };

    let osc_socket_out = UdpSocket::bind(osc_out_addr).expect( format!("Unable to provision socket: {}", osc_out_addr).as_str() );

    // build subscriber addresses
    let mut subscribers: Vec<SocketAddrV4> = Vec::with_capacity( config.subscribers.len() );
    for address in &config.subscribers {
        let socket_addr = match SocketAddrV4::from_str( format!("{}:{}", address.host, address.port).as_str() ) {
            Ok(addr)    => addr,
            Err(_)      => {
                println!("Unable to use subscriber's host and port client fields ('{}:{}') as an address!", address.host, address.port);
                ::std::process::exit(1)
            }
        };
        subscribers.push(socket_addr);
    }

    // test scene files
    for scene_file in &config.scenes {
        print!("Checking scene file: '{}'...", scene_file);
        config::check_scene_file(&scene_file).expect("Found error with scene content");
        println!("Scene OK", );
    }

    let background_scene = match config.background_scene {
        Some (ref scene_file) => Some(config::check_scene_file(scene_file).expect("Error in background scene!")),
        None => {
            println!("No background scene defined");
            None
        },
    };

    // setup metronome
    let (tx_app_msg, rx_app_msg) = mpsc::channel();
    let tx_metro = mpsc::Sender::clone(&tx_app_msg);
    let tx_osc   = mpsc::Sender::clone(&tx_app_msg);
    let metro = timer::MessageTimer::new(tx_metro);

    let step_size_ms = config.metro_step_ms as i64;
    let metro_rate = step_size_ms as i64;
    let _guard_metro = metro.schedule_repeating(chrono::Duration::milliseconds(metro_rate), AppMsg::MetroTick);

    // setup command BinaryHeap and queue first play command
    let mut future_commands = BinaryHeap::with_capacity(128);
    future_commands.push(soundscape::load_at(0, soundscape::Origin::Internal, 0));
    future_commands.push(soundscape::load_background(0));

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
                    tx_osc.send(AppMsg::Osc(action)).unwrap();
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

    let positions_limit = match config::ignore_extra_speakers(&config) {
        true => output_count,
        false => config.speaker_positions.positions.len(),
    };

    for i in 0..positions_limit {
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

    let mut elapsed_ms = 0i64;
    let mut dynamic_curve = soundscape::structure_from_scene(&open_scene(&config.scenes[0]));
    let step_increment = metro_rate as f32;

    // Schedule state
    let mut is_schedule_live = true;
    future_commands.push(soundscape::check_shedule(0));
    // Master/slave redudancy timer
    let mut is_master = false;
    let mut master_activity_timer = match config::is_fallback_slave(&config) {
        true => 1000,
        false => {
            is_master = true;
            -1
        },
    };

    if is_master {
        println!("Running as master.");
    }
    else {
        println!("Running as slave.");
    }

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
                    // add master alive message handling here
                    OscEvent::MasterAlive(new_time) => {
                        if master_activity_timer < 0 && !is_master {
                            println!("Master aquired, switching to slave mode");
                        }
                        // If we are not a master, update counters
                        if !is_master {
                            master_activity_timer = 1000;
                            elapsed_ms = new_time;
                        }
                    },
                    OscEvent::SceneChange(index, delta) => future_commands.push(soundscape::load_at(index, soundscape::Origin::Remote, delta)),
                    OscEvent::RefreshBackground => future_commands.push(soundscape::load_background(0)),
                    OscEvent::Volume (_) => println!("Ignored volume message."),
                    OscEvent::NoAction => () //println!("No action defined for {:?}", action),
                }
            }
            AppMsg::MetroTick => {
                // Keep time rolling forward if we are the master or we lose our master
                if is_master || master_activity_timer < 0 {
                    elapsed_ms += step_size_ms;
                    // trigger an update cycle
                    tx_app_msg.send(AppMsg::Update).unwrap();
                }

                if is_master {
                    // Tell any slaved device we are alive
                    let alive_message = rosc::encoder::encode(&OscPacket::Message(OscMessage {
                        addr: "/MasterAlive".to_string(),
                        args: Some( vec![ rosc::OscType::Long(elapsed_ms) ] ),
                    })).expect("Error creating Master alive message");

                    for addr in &subscribers {
                        match osc_socket_out.send_to(&alive_message, addr) {
                            Ok (_) => (),
                            Err (e) => {
                                println!("Error sending to client: {}, reason: {}", addr, e);
                                ()
                            }
                        }
                    }
                }
                else {
                    // Update slave fallover duration
                    if master_activity_timer >= 0 {
                        if master_activity_timer - metro_rate < 0 {
                            println!("Master keep alive has timed out, slave going autonomous!");
                        }
                        master_activity_timer -= metro_rate;
                    }
                }
            },
            AppMsg::Update => {
                // Update automation cycle
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
                                soundscape::Cmd::Load (n, origin) => {
                                    if origin == soundscape::Origin::Internal && !is_master && master_activity_timer > 0 {
                                        println!("Ignored local load due live remote master.");
                                    }
                                    else {
                                        println!("Executing load command at step: {}", elapsed_ms);
                                        let scene = open_scene(&config.scenes[n]);
                                        add_resources(&mut active_sources, &output_device, &scene, &speaker_positions);
                                        dynamic_curve = soundscape::structure_from_scene(&scene);

                                        future_commands.push(soundscape::play_at(elapsed_ms + step_size_ms));
                                        future_commands.push(soundscape::retire_at(elapsed_ms + scene.duration_ms));
                                        // Avoid double queueing of load actions
                                        if is_master || master_activity_timer < 0 {
                                            future_commands.push(soundscape::load_at(
                                                (n + 1) % config.scenes.len(),
                                                soundscape::Origin::Internal,
                                                elapsed_ms + scene.duration_ms + step_size_ms,
                                                ));
                                        }

                                        if is_master {
                                            // Add remote load commmand to all slaved devices
                                            let load_message = rosc::encoder::encode(&OscPacket::Message(OscMessage {
                                                addr: "/ChangeScene".to_string(),
                                                args: Some( vec!
                                                            [ rosc::OscType::Int( ((n + 1) % config.scenes.len()) as i32)
                                                            , rosc::OscType::Long(elapsed_ms + scene.duration_ms + step_size_ms)
                                                            ] ),
                                            })).expect("Error creating Master alive message");

                                            for addr in &subscribers {
                                                match osc_socket_out.send_to(&load_message, addr) {
                                                    Ok (_) => (),
                                                    Err (e) => {
                                                        println!("Error sending to client: {}, reason: {}", addr, e);
                                                        ()
                                                    }
                                                }
                                            }

                                        }
                                    }
                                },
                                soundscape::Cmd::LoadBackground => {
                                    println!("Executing LoadBackground at step: {}", elapsed_ms);
                                    if is_master {
                                        // send LoadBackground commmand to all slaved devices
                                        let load_message = rosc::encoder::encode(&OscPacket::Message(OscMessage {
                                            addr: "/RefreshBackground".to_string(),
                                            args: None
                                        })).expect("Error creating refresh background message");

                                        for addr in &subscribers {
                                            match osc_socket_out.send_to(&load_message, addr) {
                                                Ok (_) => (),
                                                Err (e) => {
                                                    println!("Error sending to client: {}, reason: {}", addr, e);
                                                    ()
                                                }
                                            }
                                        }
                                    }

                                    retire_resources(&mut background_sources, &mut retired_sources);

                                    match background_scene {
                                        Some (ref scene) => {
                                            add_resources(&mut background_sources, &output_device, &scene, &speaker_positions);
                                            play(&mut background_sources);
                                            set_volume(&mut background_sources, config.default_level);
                                        },
                                        None => (),
                                    }
                                },
                                soundscape::Cmd::Retire => {
                                    println!("Executing retire command at step: {}", elapsed_ms);
                                    retire_resources(&mut active_sources, &mut retired_sources);
                                }
                                soundscape::Cmd::CheckSchedule => {
                                    println!("Executing schedule check at step: {}", elapsed_ms);
                                    match config::is_in_schedule_now(&config, &config::localtime()) {
                                        true => {
                                            if !is_schedule_live {
                                                println!("Soundscape going live according to schedule. At {:?}", config::localtime());
                                            }
                                            is_schedule_live = true;
                                        },
                                        false => {
                                            if is_schedule_live {
                                            println!("Soundscape is going to sleep according to schedule. At {:?}", config::localtime());
                                            }
                                            is_schedule_live = false;
                                        }
                                    }
                                    // repeat the check in about 10 second
                                    future_commands.push(soundscape::check_shedule(elapsed_ms + 10_000));
                                }
                            }
                        },
                        None => println!("Expected to unpack command but no command was present. Unexpected state relating to future commands. Continuing execution."),
                    }
                }


                let t = dynamic_curve.step_t * dynamic_curve.step;
                let volume = dynamic_curve.spline.point( t );
                manage_source_activity(&mut active_sources, volume, config.default_level, master_activity_timer, is_schedule_live);
                manage_source_activity(&mut background_sources, volume, config.default_level, master_activity_timer, is_schedule_live);

                // run fades and remove any retired sources which have finished their fade out.
                for s in &mut retired_sources {
                    soundscape::update(s);
                }
                retired_sources.retain(|s| s.volume_updates > 0);

                if elapsed_ms % 3000 == 0 {
                    println!("v: {}, t: {}, step: {}, pending commands: {}", volume, t, elapsed_ms, future_commands.len());
                }
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
            else if message.addr == "/MasterAlive" {
                match message.args {
                    Some (arguments) => {
                        match arguments.first() {
                            Some(&rosc::OscType::Long (time)) => {
                                OscEvent::MasterAlive(time)
                            },
                            _ => {
                                println!("MasterAlive message requires Long");
                                OscEvent::NoAction
                            }
                        }
                    },
                    None => {
                        println!("No arguments in MasterAlive message, expected 1");
                        OscEvent::NoAction
                    },
                }
            }
            else if message.addr == "/ChangeScene" {
                match message.args {
                    Some (ref arg) => {
                        let mut arg_list = arg.iter();
                        let first = arg_list.next();
                        let second = arg_list.next();
                        if first != None && second != None {
                            let scene_index = match first.unwrap() {
                                &rosc::OscType::Int(index) => Some(index),
                                _ => None
                            };

                            let delta = match second.unwrap() {
                                &rosc::OscType::Long(delta) => Some(delta),
                                _ => None
                            };

                            if scene_index != None && delta != None {
                                OscEvent::SceneChange(scene_index.unwrap() as usize, delta.unwrap() as i64)
                            }
                            else {
                                println!("/ChangeScene requires int, int.");
                                OscEvent::NoAction
                            }
                        }
                        else {
                            OscEvent::NoAction
                        }
                    },
                    None => OscEvent::NoAction,
                }
            }
            else if message.addr == "/RefreshBackground" {
                OscEvent::RefreshBackground
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
fn manage_source_activity(sources: &mut Vec<soundscape::SoundSource>, volume :f32, default_level :f32, master_activity_timer :i64, is_schedule_live: bool) {
    for c in sources {
        soundscape::update(c); // execute volume fade steps

        if is_schedule_live && c.max_threshold > volume && c.min_threshold < volume {
            if c.is_live == false {
                c.is_live = true;
                let volume = default_level + c.gain;
                let fade_steps = c.fade_in_steps;
                soundscape::volume_fade(c, volume, fade_steps)
            }
        }
        else {
            if c.is_live == true || is_schedule_live == false {
                c.is_live = false;
                let fade_steps = c.fade_out_steps;
                soundscape::volume_fade(c, 0.0, fade_steps)
            }
        }
    }
}

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
            .fade_in(Duration::from_millis(50))
            .buffered()
            .repeat_infinite();

        // pause until a play command is executed
        sound_source.channel.set_volume(0.0);
        sound_source.channel.pause();
        sound_source.is_live = false;

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
