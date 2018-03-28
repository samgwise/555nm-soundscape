extern crate rodio;
extern crate rosc;

use rodio::Sink;
use rodio::Source;
use std::time::Duration;

use std::env;
use std::net::{UdpSocket, SocketAddrV4};
use std::str::FromStr;
use rosc::OscPacket;

enum OscEvent {
    Volume(f32),
    NoAction,
}

fn main() {
    // Handle args
    let args: Vec<String> = env::args().collect();
    let usage = format!("Usage {} <Address>:<Port>", &args[0]);

    if args.len() < 2 {
        println!("{}", usage);
        ::std::process::exit(1)
    }

    let address = match SocketAddrV4::from_str(&args[1]) {
        Ok(addr)    => addr,
        Err(_)      => panic!(usage),
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
