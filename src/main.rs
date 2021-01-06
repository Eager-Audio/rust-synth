use crossbeam::epoch::{pin, Atomic};
use std::error::Error;
use std::sync::Arc;
mod synth;
use synth::Synth;
mod midi;
use midi::{ControlChange, MidiConnection, MidiMessage};
mod audio;
use audio::Stream;
mod io_utils;

macro_rules! access_atomic {
    ($variable_name:ident) => {
        let guard = &pin();
        let mut p = $variable_name.load_consume(guard);
        let $variable_name = unsafe { p.deref_mut() };
    };
}

fn main() {
    match run() {
        Ok(_) => (),
        Err(err) => eprintln!("Error: {}", err),
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    // Audio
    //=======================================================

    let mut stream = Stream::try_new()?;

    let sample_rate = stream.sample_rate();
    let channels = stream.channels();
    let synth = Arc::new(Atomic::new(Synth::new(sample_rate)));

    let clone = Arc::clone(&synth);
    stream.output_stream(move |buffer: &mut [f32], _: &cpal::OutputCallbackInfo| {
        access_atomic!(clone);
        clone.process(channels, buffer);
    })?;

    // MIDI
    //=======================================================

    let mut listen_to_keyboard = false;

    let midi_connection = MidiConnection::try_new().map_err(|err| {
        println!("{}\nListening to keyboard...", err);
        listen_to_keyboard = true; 
        err
    });

    if let Ok(mut midi_connection) = midi_connection {
        let _connection = midi_connection.connect(
            |message, context| {
                match message {
                    MidiMessage::NoteOn(note) => {
                        access_atomic!(context);
                        context.frequency(note.frequency());
                        context.message_envelope(synth::envelope::Message::On {
                            velocity: note.gain(),
                        });
                    }
                    MidiMessage::NoteOff(_note) => {
                        access_atomic!(context);
                        context.message_envelope(synth::envelope::Message::Off);
                    }
                    MidiMessage::ControlChange(control_change) => match control_change {
                        ControlChange::Normal(channel, cc_number, value) => {}
                        ControlChange::ChannelMode(channel, cc_number, value) => {
                            unimplemented!()
                        }
                    },
                    MidiMessage::PitchBend(channel, value) => {
                        access_atomic!(context);
                        // 0 - 32767 Range
                        context.pitchbend_cents((value as f32 - 8192.0) * 0.1);
                    }
                    message => println!(
                        "Were're still working on {}!",
                        debug_struct_name(format!("{:?}", message))
                    ),
                }
            },
            Arc::clone(&synth),
        )?;
    }

    // UI
    //=======================================================

    use glutin::event::{Event, WindowEvent, ElementState};
    use glutin::event_loop::{ControlFlow, EventLoop};
    use glutin::window::WindowBuilder;

    let el = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_title("Ali and Alex's Amazing Rust Synth")
        .with_inner_size(glutin::dpi::LogicalSize::new(1024.0, 768.0));
    let windowed_context = glutin::ContextBuilder::new()
        .build_windowed(wb, &el)
        .unwrap();
    let windowed_context = unsafe { windowed_context.make_current().unwrap() };

    let synth_clone = Arc::clone(&synth);

    el.run(move |event, _, control_flow| {
        // WOW that's a lot of events :)
        //println!("{:?}", event);
        *control_flow = ControlFlow::Wait;

        match event {
            Event::LoopDestroyed => {},
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(physical_size) => windowed_context.resize(physical_size),
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput{ input, .. } => {
                    if listen_to_keyboard {
                        let note = input.scancode;
                        match input.state {
                            ElementState::Pressed => {
                                access_atomic!(synth_clone);
                                synth_clone.frequency(note as f32 * 100.0);
                                synth_clone.message_envelope(synth::envelope::Message::On {
                                    velocity: 1.0,
                                });
                            },
                            ElementState::Released => {
                                access_atomic!(synth_clone);
                                synth_clone.message_envelope(synth::envelope::Message::Off);
                            },
                        }
                    }
                }
                _ => (),
            },
            _ => (),
        }
    });

    // Thread
    //=======================================================

    // std::thread::park();
    //Ok(())
}

/// Small utility for retaining the struct name from a debug format.
/// - Disposes of all content and brackets or parenthesis
///
/// # Usage
/// ```
/// #[derive(Debug)]
/// struct AGreatTuple(f32);
/// #[derive(Debug)]
/// struct AnAwesomeStruct{ v: f32 };
///
/// let a = AGreatTuple(0.0);
/// let b = AnAwesomeStruct{ v: 0.0 };
///
/// debug_struct_name(format!("{:?}", a)); // = "AGreatTuple"
/// debug_struct_name(format!("{:?}", b)); // = "AnAwesomeStruct"
/// ```
fn debug_struct_name(mut string: String) -> String {
    let found_bracket = string.find(|c| c == '{' || c == '(');
    if let Some(index) = found_bracket {
        string = string.split_at(index).0.to_string();
    }
    string
}
