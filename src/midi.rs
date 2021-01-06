mod messages;
pub use messages::*;
use crate::io_utils::read_input;

const DEBUG_MIDI: bool = false;

use midir::{Ignore, MidiInput, MidiInputConnection, MidiInputPort};
use std::error::Error;

fn get_ports(
    midi_in: &MidiInput,
    print: bool,
) -> Result<std::vec::Vec<midir::MidiInputPort>, Box<dyn Error>> {
    if print && !midi_in.ports().is_empty() {
        println!("Available input ports:");
        for (i, p) in midi_in.ports().iter().enumerate() {
            println!("{}: {}", i, midi_in.port_name(p)?);
        }
    }

    Ok(midi_in.ports())
}

pub struct MidiConnection {
    port: MidiInputPort,
    port_name: String,
    midi_in: Option<MidiInput>,
}

fn port_prompt(ports: &[MidiInputPort], show_initial_prompt: bool) -> usize {
    let mut choice: usize = 0;
    if show_initial_prompt {
        println!("Choose your midi input.");
    }
    let input = read_input().expect("Couldn't read input.");
    match input.parse::<usize>() {
        Ok(c) => {
            if c < ports.len() {
 
                choice = c;
            } else {
                println!("{}: Not a valid port number", c);
                port_prompt(ports, false);
            }
        }
        Err(_) => {
            println!("{}: Not a valid port number", input);
            port_prompt(ports, false);
        }
    };
    choice
}

impl MidiConnection {
    pub fn try_new() -> Result<MidiConnection, Box<dyn Error>> {
        let mut midi_in = MidiInput::new("rust-synth input")?;
        midi_in.ignore(Ignore::None);

        let ports = get_ports(&midi_in, true)?;
        let port = port_prompt(&ports, true);

        let port = ports.get(port).ok_or("port not found")?.clone();
        let port_name = midi_in.port_name(&port)?;

        println!("Selected: {}", port_name);

        Ok(MidiConnection {
            port,
            port_name,
            midi_in: Some(midi_in),
        })
    }

    pub fn connect<F, T: Send + 'static>(
        &mut self,
        mut callback: F,
        context: T,
    ) -> Result<MidiInputConnection<T>, Box<dyn Error>>
    where
        F: FnMut(MidiMessage, &mut T) + Send + 'static,
    {
        let connection_status = self
            .midi_in
            .take()
            .ok_or("A connection is already open")?
            .connect(
                &self.port,
                &self.port_name,
                move |micros, raw_message, context| {
                    let message = MidiMessage::try_new(raw_message);
                    if DEBUG_MIDI {
                        println!("=============================\n");
                        println!("Microseconds: {}\n", micros);
                        println!("Raw Message: {:?}\n", raw_message);
                        println!("Message: {:#?}\n", message);
                    }
                    match MidiMessage::try_new(raw_message) {
                        Err(err) => eprintln!("{}", err),
                        Ok(message) => callback(message, context),
                    }
                },
                context,
            );
        Ok(connection_status?)
    }
}
