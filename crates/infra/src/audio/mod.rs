//! Platform-specific audio backend implementations using CPAL
//!
//! This module provides cross-platform audio support through CPAL, which abstracts
//! platform-specific APIs:
//! - Windows: WASAPI
//! - Linux: ALSA/PulseAudio
//! - macOS: CoreAudio

pub mod cpal_backend;
pub mod engine;
pub mod lockfree_buffer;
pub mod stream;

pub use cpal_backend::*;
pub use engine::*;
pub use lockfree_buffer::*;
pub use stream::*;
