#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;

extern crate rosc;
use rosc::OscPacket;

extern crate rodio;
use rodio::Sink;
use rodio::Source;
use std::time::Duration;

use std::env;
use std::net::{UdpSocket, SocketAddrV4};
use std::str::FromStr;

mod config;

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

    // Setup audio
    let channel_count = 16;
    let voices = channel_count as f32;

    let endpoint = rodio::default_endpoint().unwrap();

    let mut channel = Vec::new();
    for n in 0..(channel_count - 1) {
        let overtone = n as u32;
        // let voice = n as f32;
        let delay = n as u64;
        channel.push( Sink::new(&endpoint) );

        let source =
            rodio::source::SineWave::new(110u32 * overtone)
            .amplify( (0.25 / voices) )
            .fade_in(Duration::from_millis(200 * delay));
        channel[n].append(source);
    }

    // Setup socket
    let socket = UdpSocket::bind(address).unwrap();
    println!("Listening on {}...", address);

    // Block while listening for OSC messages
    let mut packet_buffer = [0u8; rosc::decoder::MTU];

    loop {
        match socket.recv_from(&mut packet_buffer) {
            Ok((bytes, _remote_address)) => {
                let routing = route_osc(
                    rosc::decoder::decode(&packet_buffer[..bytes]).unwrap()
                );

                match routing {
                    OscEvent::Volume(volume) => set_volume(&mut channel, volume),
                    OscEvent::NoAction       => println!("No action defined."),
                }
            }
            Err(e) => {
                // Log to console and quit the recv loop
                println!("Error receiving from socket: {}", e);
                break;
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

fn set_volume(channels: &mut Vec<Sink>, volume: f32) {
    for c in channels {
        c.set_volume(volume)
    }
}
