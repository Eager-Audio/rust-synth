pub const NOTE_OFF: u8 = 0b1000;
pub const NOTE_ON: u8 = 0b1001;
pub const _POLYPHONIC_AFTER_TOUCH: u8 = 0b1010;
pub const CONTROL_CHANGE: u8 = 0b1011;
pub const PROGRAM_CHANGE: u8 = 0b1100;
pub const _AFTER_TOUCH: u8 = 0b1101;
pub const PITCH_BEND_CHANGE: u8 = 0b1110;

#[derive(Debug)]
pub struct Note(u8, u8, u8);

impl Note {
    pub fn frequency(&self) -> f32 {
        let note = self.1 as f32;
        let base_frequency = 440.0;
        2.0_f32.powf((note - 69.0) / 12.0) * base_frequency
    }

    pub fn gain(&self) -> f32 {
        let velocity = self.2 as f32;
        let db = velocity / 127.0 * 70.0 - 70.0;
        10.0_f32.powf(db / 20.0)
    }
}
#[derive(Debug)]
pub enum ControlChange {
    Normal(u8, u8, u8),
    ChannelMode(u8, u8, u8),
}

#[derive(Debug)]
pub enum MidiMessage {
    NoteOn(Note),
    NoteOff(Note),
    ProgramChange(u8, u8),
    ControlChange(ControlChange),
    PitchBend(u8, u16),
}

fn split_status_and_channel(status_byte: u8) -> (u8, u8) {
    let channel = status_byte & 0b00001111;
    let status = status_byte >> 4;
    (status, channel)
}

impl MidiMessage {
    pub fn try_new(raw_message: &[u8]) -> Result<Self, &str> {
        use MidiMessage::*;
        let (status, channel) = split_status_and_channel(raw_message[0]);

        match status {
            NOTE_OFF => Ok(NoteOff(Note(channel, raw_message[1], raw_message[2]))),
            NOTE_ON => Ok(NoteOn(Note(channel, raw_message[1], raw_message[2]))),
            CONTROL_CHANGE => {
                use self::ControlChange::*;
                let cc_number = raw_message[1];
                Ok(ControlChange(if cc_number <= 119 {
                    Normal(channel, raw_message[1], raw_message[2])
                } else {
                    ChannelMode(channel, raw_message[1], raw_message[2])
                }))
            }
            PITCH_BEND_CHANGE => {
                let msb = (raw_message[2] as u16) << 7;
                let lsb = raw_message[1] as u16;
                let pitchbend_value = msb | lsb;
                Ok(PitchBend(channel, pitchbend_value))
            }
            PROGRAM_CHANGE => Ok(ProgramChange(channel, raw_message[1])),
            _ => Err("Unrecognized message"),
        }
    }
}