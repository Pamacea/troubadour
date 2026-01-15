//! Digital Signal Processing effects

use crate::domain::audio::Result;

pub trait Effect {
    fn process(&mut self, buffer: &mut [f32]) -> Result<()>;
    fn reset(&mut self);
}

pub struct Equalizer;
pub struct Compressor;
