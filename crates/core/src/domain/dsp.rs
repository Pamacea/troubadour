//! Digital Signal Processing effects for audio processing
//!
//! This module provides a suite of audio effects including:
//! - 3-band Equalizer (biquad-based IIR filters)
//! - Dynamic Compressor
//! - Noise Gate with sidechain support
//!
//! All effects are designed for:
//! - Zero allocations in the hot path
//! - SIMD-friendly processing
//! - Minimal latency (< 1ms)
//! - < 1% CPU per active effect

use crate::domain::audio::AudioError;
use serde::{Deserialize, Serialize};
use tracing::trace;

pub type Result<T> = std::result::Result<T, AudioError>;

/// Core trait for all audio effects
///
/// All effects process audio in-place on f32 buffers normalized to [-1.0, 1.0].
pub trait Effect: Send + Sync {
    /// Process a buffer of audio samples in-place
    ///
    /// # Requirements
    /// - No allocations in the hot path
    /// - SIMD-friendly when possible
    /// - Handle buffer of any size
    fn process(&mut self, buffer: &mut [f32]) -> Result<()>;

    /// Reset effect state to initial conditions
    ///
    /// Clears internal buffers and state. Useful for:
    /// - Preventing clicks on parameter changes
    /// - Clearing delay lines on transport stop
    fn reset(&mut self);

    /// Check if effect is bypassed (zero processing overhead when true)
    fn is_bypassed(&self) -> bool;

    /// Toggle bypass state
    fn set_bypass(&mut self, bypass: bool);

    /// Get effect name for debugging/display
    fn name(&self) -> &str;
}

/// Parameter constraints for DSP effects
///
/// All parameters are clamped to these ranges to prevent
/// invalid states and ensure numerical stability.
pub mod params {
    /// Decibel range for gain-based parameters
    pub const DB_MIN: f32 = -60.0;
    pub const DB_MAX: f32 = 24.0;

    /// Compressor ratio range (1:1 to 20:1)
    pub const RATIO_MIN: f32 = 1.0;
    pub const RATIO_MAX: f32 = 20.0;

    /// Attack/Release time ranges in seconds
    pub const ATTACK_MIN: f32 = 0.0001;  // 0.1ms
    pub const ATTACK_MAX: f32 = 0.1;     // 100ms
    pub const RELEASE_MIN: f32 = 0.01;   // 10ms
    pub const RELEASE_MAX: f32 = 1.0;    // 1000ms

    /// Hold time for gate (in seconds)
    pub const HOLD_MIN: f32 = 0.0;
    pub const HOLD_MAX: f32 = 2.0;

    /// EQ frequency ranges (Hz)
    pub const FREQ_LOW_SHELF: f32 = 200.0;
    pub const FREQ_MID_CENTER: f32 = 1000.0;
    pub const FREQ_HIGH_SHELF: f32 = 2000.0;
}

// ============================================================================
// BIQUAD FILTER (Low-level IIR filter for EQ)
// ============================================================================

/// Biquad filter coefficients
///
/// Direct Form I implementation for numerical stability.
/// Coefficients are pre-computed to avoid per-sample calculations.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BiquadCoeffs {
    /// Numerator coefficients
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    /// Denominator coefficients (a0 is normalized to 1.0)
    pub a1: f32,
    pub a2: f32,
}

impl Default for BiquadCoeffs {
    fn default() -> Self {
        // Unity gain (no filtering)
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}

impl BiquadCoeffs {
    /// Calculate coefficients for a low shelf filter
    ///
    /// Boosts or cuts frequencies below the cutoff frequency.
    ///
    /// # Parameters
    /// - `sample_rate`: Audio sample rate in Hz
    /// - `freq`: Corner frequency in Hz
    /// - `gain_db`: Boost/cut in decibels (clamped to +/- 12dB)
    /// - `q`: Q factor (resonance), typically 0.5-1.0
    #[must_use]
    pub fn low_shelf(sample_rate: f32, freq: f32, gain_db: f32, q: f32) -> Self {
        let gain_db = gain_db.clamp(-12.0, 12.0);
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);

        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
        let a1 = 2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;

        // Normalize by a0
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// Calculate coefficients for a high shelf filter
    ///
    /// Boosts or cuts frequencies above the cutoff frequency.
    #[must_use]
    pub fn high_shelf(sample_rate: f32, freq: f32, gain_db: f32, q: f32) -> Self {
        let gain_db = gain_db.clamp(-12.0, 12.0);
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);

        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
        let a1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// Calculate coefficients for a peaking EQ filter
    ///
    /// Boosts or cuts frequencies around a center frequency.
    #[must_use]
    pub fn peaking(sample_rate: f32, freq: f32, gain_db: f32, q: f32) -> Self {
        let gain_db = gain_db.clamp(-12.0, 12.0);
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;

        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }
}

/// Stateful biquad filter using Direct Form I
///
/// Direct Form I is chosen over Transposed Direct Form II for:
/// - Better numerical stability with low-frequency filters
/// - Easier coefficient updates without artifacts
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BiquadFilter {
    coeffs: BiquadCoeffs,
    // Previous input samples (x[n-1], x[n-2])
    x1: f32,
    x2: f32,
    // Previous output samples (y[n-1], y[n-2])
    y1: f32,
    y2: f32,
}

impl BiquadFilter {
    /// Create a new biquad filter with given coefficients
    pub fn new(coeffs: BiquadCoeffs) -> Self {
        Self {
            coeffs,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    /// Create a bypass filter (unity gain)
    pub fn bypass() -> Self {
        Self::new(BiquadCoeffs::default())
    }

    /// Update filter coefficients
    ///
    /// Can be called in real-time for parameter changes.
    pub fn set_coeffs(&mut self, coeffs: BiquadCoeffs) {
        self.coeffs = coeffs;
    }

    /// Process a single sample
    #[inline]
    fn process_sample(&mut self, x: f32) -> f32 {
        // Direct Form I: y[n] = b0*x[n] + b1*x[n-1] + b2*x[n-2]
        //                        - a1*y[n-1] - a2*y[n-2]
        let y = self.coeffs.b0 * x
            + self.coeffs.b1 * self.x1
            + self.coeffs.b2 * self.x2
            - self.coeffs.a1 * self.y1
            - self.coeffs.a2 * self.y2;

        // Update state
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;

        y
    }

    /// Process a buffer of samples
    pub fn process(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

// ============================================================================
// 3-BAND EQUALIZER
// ============================================================================

/// 3-band parametric equalizer
///
/// Three independent bands:
/// - Low shelf: frequencies below 200 Hz
/// - Mid peaking: 200 Hz - 2 kHz (adjustable center frequency)
/// - High shelf: frequencies above 2 kHz
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Equalizer {
    bypass: bool,
    sample_rate: f32,
    // Filters (stereo)
    low_left: BiquadFilter,
    low_right: BiquadFilter,
    mid_left: BiquadFilter,
    mid_right: BiquadFilter,
    high_left: BiquadFilter,
    high_right: BiquadFilter,
    // Parameters
    low_gain_db: f32,
    mid_freq: f32,
    mid_gain_db: f32,
    mid_q: f32,
    high_gain_db: f32,
}

impl Equalizer {
    /// Default frequency for the low shelf
    pub const DEFAULT_LOW_FREQ: f32 = 200.0;
    /// Default center frequency for mid band
    pub const DEFAULT_MID_FREQ: f32 = 1000.0;
    /// Default frequency for the high shelf
    pub const DEFAULT_HIGH_FREQ: f32 = 2000.0;

    /// Create a new 3-band equalizer
    pub fn new(sample_rate: u32) -> Self {
        let sr = sample_rate as f32;
        let bypass_coeff = BiquadCoeffs::default();

        Self {
            bypass: false,
            sample_rate: sr,
            low_left: BiquadFilter::new(bypass_coeff),
            low_right: BiquadFilter::new(bypass_coeff),
            mid_left: BiquadFilter::new(bypass_coeff),
            mid_right: BiquadFilter::new(bypass_coeff),
            high_left: BiquadFilter::new(bypass_coeff),
            high_right: BiquadFilter::new(bypass_coeff),
            low_gain_db: 0.0,
            mid_freq: Self::DEFAULT_MID_FREQ,
            mid_gain_db: 0.0,
            mid_q: 1.0,
            high_gain_db: 0.0,
        }
    }

    /// Set the low shelf gain
    pub fn set_low_gain(&mut self, gain_db: f32) {
        self.low_gain_db = gain_db.clamp(-12.0, 12.0);
        self.update_coefficients();
    }

    /// Set the mid band center frequency
    pub fn set_mid_freq(&mut self, freq: f32) {
        self.mid_freq = freq.clamp(200.0, 2000.0);
        self.update_coefficients();
    }

    /// Set the mid band gain
    pub fn set_mid_gain(&mut self, gain_db: f32) {
        self.mid_gain_db = gain_db.clamp(-12.0, 12.0);
        self.update_coefficients();
    }

    /// Set the mid band Q factor
    pub fn set_mid_q(&mut self, q: f32) {
        self.mid_q = q.clamp(0.1, 5.0);
        self.update_coefficients();
    }

    /// Set the high shelf gain
    pub fn set_high_gain(&mut self, gain_db: f32) {
        self.high_gain_db = gain_db.clamp(-12.0, 12.0);
        self.update_coefficients();
    }

    /// Update all filter coefficients based on current parameters
    fn update_coefficients(&mut self) {
        let low = BiquadCoeffs::low_shelf(self.sample_rate, Self::DEFAULT_LOW_FREQ, self.low_gain_db, 0.707);
        let mid = BiquadCoeffs::peaking(self.sample_rate, self.mid_freq, self.mid_gain_db, self.mid_q);
        let high = BiquadCoeffs::high_shelf(self.sample_rate, Self::DEFAULT_HIGH_FREQ, self.high_gain_db, 0.707);

        self.low_left.set_coeffs(low);
        self.low_right.set_coeffs(low);
        self.mid_left.set_coeffs(mid);
        self.mid_right.set_coeffs(mid);
        self.high_left.set_coeffs(high);
        self.high_right.set_coeffs(high);

        trace!(
            "EQ updated: L={:.1}dB, M={:.1}dB@{:.0}Hz, H={:.1}dB",
            self.low_gain_db,
            self.mid_gain_db,
            self.mid_freq,
            self.high_gain_db
        );
    }

    /// Get current parameter values
    pub fn params(&self) -> EqualizerParams {
        EqualizerParams {
            low_gain_db: self.low_gain_db,
            mid_freq: self.mid_freq,
            mid_gain_db: self.mid_gain_db,
            mid_q: self.mid_q,
            high_gain_db: self.high_gain_db,
        }
    }

    /// Set all parameters at once
    pub fn set_params(&mut self, params: EqualizerParams) {
        self.low_gain_db = params.low_gain_db.clamp(-12.0, 12.0);
        self.mid_freq = params.mid_freq.clamp(200.0, 2000.0);
        self.mid_gain_db = params.mid_gain_db.clamp(-12.0, 12.0);
        self.mid_q = params.mid_q.clamp(0.1, 5.0);
        self.high_gain_db = params.high_gain_db.clamp(-12.0, 12.0);
        self.update_coefficients();
    }
}

impl Effect for Equalizer {
    fn process(&mut self, buffer: &mut [f32]) -> Result<()> {
        if self.bypass {
            return Ok(());
        }

        // Process as interleaved stereo
        // We can optimize this with SIMD later, but for now use simple loop
        let channels = 2;
        for i in 0..buffer.len() / channels {
            let left_idx = i * channels;
            let right_idx = left_idx + 1;

            if right_idx < buffer.len() {
                // Process left channel
                buffer[left_idx] = self.low_left.process_sample(buffer[left_idx]);
                buffer[left_idx] = self.mid_left.process_sample(buffer[left_idx]);
                buffer[left_idx] = self.high_left.process_sample(buffer[left_idx]);

                // Process right channel
                buffer[right_idx] = self.low_right.process_sample(buffer[right_idx]);
                buffer[right_idx] = self.mid_right.process_sample(buffer[right_idx]);
                buffer[right_idx] = self.high_right.process_sample(buffer[right_idx]);
            } else if left_idx < buffer.len() {
                // Mono - just use left filters
                buffer[left_idx] = self.low_left.process_sample(buffer[left_idx]);
                buffer[left_idx] = self.mid_left.process_sample(buffer[left_idx]);
                buffer[left_idx] = self.high_left.process_sample(buffer[left_idx]);
            }
        }

        Ok(())
    }

    fn reset(&mut self) {
        self.low_left.reset();
        self.low_right.reset();
        self.mid_left.reset();
        self.mid_right.reset();
        self.high_left.reset();
        self.high_right.reset();
    }

    fn is_bypassed(&self) -> bool {
        self.bypass
    }

    fn set_bypass(&mut self, bypass: bool) {
        self.bypass = bypass;
        if bypass {
            self.reset();
        }
    }

    fn name(&self) -> &str {
        "Equalizer"
    }
}

/// Equalizer parameters
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EqualizerParams {
    pub low_gain_db: f32,
    pub mid_freq: f32,
    pub mid_gain_db: f32,
    pub mid_q: f32,
    pub high_gain_db: f32,
}

impl Default for EqualizerParams {
    fn default() -> Self {
        Self {
            low_gain_db: 0.0,
            mid_freq: 1000.0,
            mid_gain_db: 0.0,
            mid_q: 1.0,
            high_gain_db: 0.0,
        }
    }
}

// ============================================================================
// DYNAMIC RANGE COMPRESSOR
// ============================================================================

/// Dynamic range compressor
///
/// Reduces the dynamic range of audio signals by attenuating
/// signals above a threshold. Includes smooth gain reduction
/// to avoid pumping artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Compressor {
    bypass: bool,
    // Parameters
    threshold_db: f32,
    ratio: f32,
    attack_sec: f32,
    release_sec: f32,
    makeup_gain_db: f32,
    // Coefficients (pre-computed for performance)
    attack_coeff: f32,
    release_coeff: f32,
    // Envelope follower state (stereo)
    envelope_left: f32,
    envelope_right: f32,
}

impl Compressor {
    /// Create a new compressor with default parameters
    pub fn new(sample_rate: u32) -> Self {
        let mut comp = Self {
            bypass: false,
            threshold_db: -18.0,
            ratio: 4.0,
            attack_sec: 0.005,  // 5ms
            release_sec: 0.1,   // 100ms
            makeup_gain_db: 0.0,
            attack_coeff: 0.0,
            release_coeff: 0.0,
            envelope_left: 0.0,
            envelope_right: 0.0,
        };
        comp.update_coefficients(sample_rate as f32);
        comp
    }

    /// Set the threshold in dB
    pub fn set_threshold(&mut self, threshold_db: f32, sample_rate: u32) {
        self.threshold_db = threshold_db.clamp(params::DB_MIN, 0.0);
        self.update_coefficients(sample_rate as f32);
    }

    /// Set the compression ratio (1:1 = no compression, 20:1 = limiter)
    pub fn set_ratio(&mut self, ratio: f32) {
        self.ratio = ratio.clamp(params::RATIO_MIN, params::RATIO_MAX);
    }

    /// Set the attack time in seconds
    pub fn set_attack(&mut self, attack_sec: f32, sample_rate: u32) {
        self.attack_sec = attack_sec.clamp(params::ATTACK_MIN, params::ATTACK_MAX);
        self.update_coefficients(sample_rate as f32);
    }

    /// Set the release time in seconds
    pub fn set_release(&mut self, release_sec: f32, sample_rate: u32) {
        self.release_sec = release_sec.clamp(params::RELEASE_MIN, params::RELEASE_MAX);
        self.update_coefficients(sample_rate as f32);
    }

    /// Set the make-up gain in dB
    pub fn set_makeup_gain(&mut self, gain_db: f32) {
        self.makeup_gain_db = gain_db.clamp(0.0, params::DB_MAX);
    }

    /// Update envelope filter coefficients
    fn update_coefficients(&mut self, sample_rate: f32) {
        // Convert time constants to filter coefficients
        // Using exp(-1/(time * sample_rate)) for smooth envelope following
        self.attack_coeff = (-1.0 / (self.attack_sec * sample_rate)).exp();
        self.release_coeff = (-1.0 / (self.release_sec * sample_rate)).exp();
    }

    /// Calculate gain reduction for a given input level
    ///
    /// Returns the linear gain to apply (1.0 = no reduction)
    #[inline]
    fn calculate_gain(&self, input_level_db: f32) -> f32 {
        if input_level_db <= self.threshold_db {
            return 1.0;
        }

        // Calculate gain reduction using the "soft knee" formula
        // This provides a smoother transition around the threshold
        let over_threshold = input_level_db - self.threshold_db;
        let gain_reduction_db = over_threshold * (1.0 - 1.0 / self.ratio);

        // Convert to linear gain
        10.0_f32.powf(-gain_reduction_db / 20.0)
    }

    /// Update envelope follower (peak detection with smoothing)
    #[inline]
    fn update_envelope(&mut self, input_sample: f32, envelope: f32) -> f32 {
        let input_level = input_sample.abs();

        // Use attack coefficient for rising, release for falling
        let coeff = if input_level > envelope {
            self.attack_coeff
        } else {
            self.release_coeff
        };

        coeff * envelope + (1.0 - coeff) * input_level
    }

    /// Convert linear amplitude to dB (with minimum floor)
    #[inline]
    fn to_db(level: f32) -> f32 {
        if level < 1e-6 {
            params::DB_MIN
        } else {
            20.0 * level.log10()
        }
    }

    /// Get current parameter values
    pub fn params(&self) -> CompressorParams {
        CompressorParams {
            threshold_db: self.threshold_db,
            ratio: self.ratio,
            attack_sec: self.attack_sec,
            release_sec: self.release_sec,
            makeup_gain_db: self.makeup_gain_db,
        }
    }

    /// Set all parameters at once
    pub fn set_params(&mut self, params: CompressorParams, sample_rate: u32) {
        self.threshold_db = params.threshold_db.clamp(params::DB_MIN, 0.0);
        self.ratio = params.ratio.clamp(params::RATIO_MIN, params::RATIO_MAX);
        self.attack_sec = params.attack_sec.clamp(params::ATTACK_MIN, params::ATTACK_MAX);
        self.release_sec = params.release_sec.clamp(params::RELEASE_MIN, params::RELEASE_MAX);
        self.makeup_gain_db = params.makeup_gain_db.clamp(0.0, params::DB_MAX);
        self.update_coefficients(sample_rate as f32);
    }
}

impl Effect for Compressor {
    fn process(&mut self, buffer: &mut [f32]) -> Result<()> {
        if self.bypass {
            return Ok(());
        }

        // Pre-compute makeup gain
        let makeup_gain = 10.0_f32.powf(self.makeup_gain_db / 20.0);

        // Process as interleaved stereo
        let channels = 2;
        for i in 0..buffer.len() / channels {
            let left_idx = i * channels;
            let right_idx = left_idx + 1;

            // Process left channel
            if left_idx < buffer.len() {
                self.envelope_left = self.update_envelope(buffer[left_idx], self.envelope_left);
                let envelope_db = Self::to_db(self.envelope_left);
                let gain = self.calculate_gain(envelope_db);
                buffer[left_idx] *= gain * makeup_gain;
            }

            // Process right channel
            if right_idx < buffer.len() {
                self.envelope_right = self.update_envelope(buffer[right_idx], self.envelope_right);
                let envelope_db = Self::to_db(self.envelope_right);
                let gain = self.calculate_gain(envelope_db);
                buffer[right_idx] *= gain * makeup_gain;
            }
        }

        Ok(())
    }

    fn reset(&mut self) {
        self.envelope_left = 0.0;
        self.envelope_right = 0.0;
    }

    fn is_bypassed(&self) -> bool {
        self.bypass
    }

    fn set_bypass(&mut self, bypass: bool) {
        self.bypass = bypass;
        if bypass {
            self.reset();
        }
    }

    fn name(&self) -> &str {
        "Compressor"
    }
}

/// Compressor parameters
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CompressorParams {
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_sec: f32,
    pub release_sec: f32,
    pub makeup_gain_db: f32,
}

impl Default for CompressorParams {
    fn default() -> Self {
        Self {
            threshold_db: -18.0,
            ratio: 4.0,
            attack_sec: 0.005,
            release_sec: 0.1,
            makeup_gain_db: 0.0,
        }
    }
}

// ============================================================================
// NOISE GATE
// ============================================================================

/// Noise gate with sidechain support
///
/// Silences audio signals below a threshold, useful for
/// eliminating background noise when no signal is present.
#[derive(Debug, Serialize, Deserialize)]
pub struct NoiseGate {
    bypass: bool,
    // Parameters
    threshold_db: f32,
    attack_sec: f32,
    release_sec: f32,
    hold_sec: f32,
    // Envelope/gain state (stereo) - tracked across process calls
    envelope_left: f32,
    envelope_right: f32,
    gain_left: f32,
    gain_right: f32,
    // Hold counters (stereo)
    hold_counter_left: u32,
    hold_counter_right: u32,
    // Coefficients
    attack_coeff: f32,
    release_coeff: f32,
    // Sidechain buffer (optional external signal for triggering)
    use_sidechain: bool,
    sidechain_level: f32,
}

// Manual Clone implementation - reset state when cloning
impl Clone for NoiseGate {
    fn clone(&self) -> Self {
        Self {
            bypass: self.bypass,
            threshold_db: self.threshold_db,
            attack_sec: self.attack_sec,
            release_sec: self.release_sec,
            hold_sec: self.hold_sec,
            envelope_left: 0.0,
            envelope_right: 0.0,
            gain_left: 0.0,
            gain_right: 0.0,
            hold_counter_left: 0,
            hold_counter_right: 0,
            attack_coeff: self.attack_coeff,
            release_coeff: self.release_coeff,
            use_sidechain: self.use_sidechain,
            sidechain_level: self.sidechain_level,
        }
    }
}

impl NoiseGate {
    /// Create a new noise gate
    pub fn new(sample_rate: u32) -> Self {
        let mut gate = Self {
            bypass: false,
            threshold_db: -40.0,
            attack_sec: 0.001,   // 1ms
            release_sec: 0.05,   // 50ms
            hold_sec: 0.1,       // 100ms hold time
            envelope_left: 0.0,
            envelope_right: 0.0,
            gain_left: 0.0,
            gain_right: 0.0,
            hold_counter_left: 0,
            hold_counter_right: 0,
            attack_coeff: 0.0,
            release_coeff: 0.0,
            use_sidechain: false,
            sidechain_level: params::DB_MIN,
        };
        gate.update_coefficients(sample_rate as f32);
        gate
    }

    /// Set the gate threshold in dB
    pub fn set_threshold(&mut self, threshold_db: f32, sample_rate: u32) {
        self.threshold_db = threshold_db.clamp(params::DB_MIN, 0.0);
        self.update_coefficients(sample_rate as f32);
    }

    /// Set the attack time in seconds
    pub fn set_attack(&mut self, attack_sec: f32, sample_rate: u32) {
        self.attack_sec = attack_sec.clamp(params::ATTACK_MIN, params::ATTACK_MAX);
        self.update_coefficients(sample_rate as f32);
    }

    /// Set the release time in seconds
    pub fn set_release(&mut self, release_sec: f32, sample_rate: u32) {
        self.release_sec = release_sec.clamp(params::RELEASE_MIN, params::RELEASE_MAX);
        self.update_coefficients(sample_rate as f32);
    }

    /// Set the hold time in seconds
    pub fn set_hold(&mut self, hold_sec: f32) {
        self.hold_sec = hold_sec.clamp(params::HOLD_MIN, params::HOLD_MAX);
    }

    /// Enable or disable sidechain mode
    ///
    /// When enabled, the gate uses an external signal (set via `set_sidechain_level`)
    /// to determine when to open, rather than the input signal itself.
    pub fn set_sidechain(&mut self, enabled: bool) {
        self.use_sidechain = enabled;
    }

    /// Set the sidechain signal level (for external triggering)
    ///
    /// This should be updated each buffer with the level of the external signal.
    pub fn set_sidechain_level(&mut self, level_db: f32) {
        self.sidechain_level = level_db;
    }

    /// Update filter coefficients
    fn update_coefficients(&mut self, sample_rate: f32) {
        self.attack_coeff = (-1.0 / (self.attack_sec * sample_rate)).exp();
        self.release_coeff = (-1.0 / (self.release_sec * sample_rate)).exp();
    }

    /// Get current parameter values
    pub fn params(&self) -> NoiseGateParams {
        NoiseGateParams {
            threshold_db: self.threshold_db,
            attack_sec: self.attack_sec,
            release_sec: self.release_sec,
            hold_sec: self.hold_sec,
            use_sidechain: self.use_sidechain,
        }
    }

    /// Set all parameters at once
    pub fn set_params(&mut self, params: NoiseGateParams, sample_rate: u32) {
        self.threshold_db = params.threshold_db.clamp(params::DB_MIN, 0.0);
        self.attack_sec = params.attack_sec.clamp(params::ATTACK_MIN, params::ATTACK_MAX);
        self.release_sec = params.release_sec.clamp(params::RELEASE_MIN, params::RELEASE_MAX);
        self.hold_sec = params.hold_sec.clamp(params::HOLD_MIN, params::HOLD_MAX);
        self.use_sidechain = params.use_sidechain;
        self.update_coefficients(sample_rate as f32);
    }
}

impl Effect for NoiseGate {
    fn process(&mut self, buffer: &mut [f32]) -> Result<()> {
        if self.bypass {
            return Ok(());
        }

        // Calculate hold time in samples
        let hold_samples = (self.hold_sec * 48000.0) as u32;

        // Process as interleaved stereo
        let channels = 2;
        for i in 0..buffer.len() / channels {
            let left_idx = i * channels;
            let right_idx = left_idx + 1;

            // Process left channel
            if left_idx < buffer.len() {
                let input_db = if self.use_sidechain {
                    self.sidechain_level
                } else {
                    Compressor::to_db(buffer[left_idx].abs())
                };

                // Update envelope follower
                let target_env = if input_db > self.threshold_db { 1.0 } else { 0.0 };
                let coeff = if target_env > self.envelope_left {
                    self.attack_coeff
                } else {
                    self.release_coeff
                };
                self.envelope_left = coeff * self.envelope_left + (1.0 - coeff) * target_env;

                // Update hold counter
                if self.envelope_left > 0.5 {
                    self.hold_counter_left = hold_samples;
                } else if self.hold_counter_left > 0 {
                    self.hold_counter_left -= 1;
                }

                // Calculate gain: open if envelope is high or we're in hold period
                let target_gain = if self.envelope_left > 0.5 || self.hold_counter_left > 0 {
                    1.0
                } else {
                    0.0
                };

                // Smooth gain transition
                let gain_coeff = if target_gain > self.gain_left {
                    self.attack_coeff
                } else {
                    self.release_coeff
                };
                self.gain_left = gain_coeff * self.gain_left + (1.0 - gain_coeff) * target_gain;

                buffer[left_idx] *= self.gain_left;
            }

            // Process right channel
            if right_idx < buffer.len() {
                let input_db = if self.use_sidechain {
                    self.sidechain_level
                } else {
                    Compressor::to_db(buffer[right_idx].abs())
                };

                // Update envelope follower
                let target_env = if input_db > self.threshold_db { 1.0 } else { 0.0 };
                let coeff = if target_env > self.envelope_right {
                    self.attack_coeff
                } else {
                    self.release_coeff
                };
                self.envelope_right = coeff * self.envelope_right + (1.0 - coeff) * target_env;

                // Update hold counter
                if self.envelope_right > 0.5 {
                    self.hold_counter_right = hold_samples;
                } else if self.hold_counter_right > 0 {
                    self.hold_counter_right -= 1;
                }

                // Calculate gain: open if envelope is high or we're in hold period
                let target_gain = if self.envelope_right > 0.5 || self.hold_counter_right > 0 {
                    1.0
                } else {
                    0.0
                };

                // Smooth gain transition
                let gain_coeff = if target_gain > self.gain_right {
                    self.attack_coeff
                } else {
                    self.release_coeff
                };
                self.gain_right = gain_coeff * self.gain_right + (1.0 - gain_coeff) * target_gain;

                buffer[right_idx] *= self.gain_right;
            }
        }

        Ok(())
    }

    fn reset(&mut self) {
        self.envelope_left = 0.0;
        self.envelope_right = 0.0;
        self.gain_left = 0.0;
        self.gain_right = 0.0;
        self.hold_counter_left = 0;
        self.hold_counter_right = 0;
        self.sidechain_level = params::DB_MIN;
    }

    fn is_bypassed(&self) -> bool {
        self.bypass
    }

    fn set_bypass(&mut self, bypass: bool) {
        self.bypass = bypass;
        if bypass {
            self.reset();
        }
    }

    fn name(&self) -> &str {
        "NoiseGate"
    }
}

/// Noise gate parameters
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NoiseGateParams {
    pub threshold_db: f32,
    pub attack_sec: f32,
    pub release_sec: f32,
    pub hold_sec: f32,
    pub use_sidechain: bool,
}

impl Default for NoiseGateParams {
    fn default() -> Self {
        Self {
            threshold_db: -40.0,
            attack_sec: 0.001,
            release_sec: 0.05,
            hold_sec: 0.1,
            use_sidechain: false,
        }
    }
}

// ============================================================================
// EFFECTS CHAIN
// ============================================================================

/// Serial effects chain for processing audio through multiple effects
///
/// Effects are processed in the order they were added.
/// Bypassed effects are skipped with zero overhead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectsChain {
    effects: Vec<EffectType>,
}

/// Serializable wrapper for effects
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum EffectType {
    Equalizer(EqualizerParams),
    Compressor(CompressorParams),
    NoiseGate(NoiseGateParams),
}

impl EffectType {
    /// Get the effect name
    pub fn name(&self) -> &str {
        match self {
            EffectType::Equalizer(_) => "Equalizer",
            EffectType::Compressor(_) => "Compressor",
            EffectType::NoiseGate(_) => "NoiseGate",
        }
    }
}

impl EffectsChain {
    /// Create a new empty effects chain
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
        }
    }

    /// Add an effect to the end of the chain
    pub fn add(&mut self, effect: EffectType) {
        self.effects.push(effect);
    }

    /// Remove an effect by index
    pub fn remove(&mut self, index: usize) -> Result<()> {
        if index < self.effects.len() {
            self.effects.remove(index);
            Ok(())
        } else {
            Err(AudioError::InvalidConfiguration(
                "Effect index out of bounds".to_string(),
            ))
        }
    }

    /// Get the number of effects in the chain
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Check if the chain is empty
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Get all effect configurations
    pub fn effects(&self) -> &[EffectType] {
        &self.effects
    }

    /// Clear all effects
    pub fn clear(&mut self) {
        self.effects.clear();
    }

    /// Create a runtime processor from this chain configuration
    pub fn create_processor(&self, sample_rate: u32) -> EffectsChainProcessor {
        let processors: Vec<Box<dyn Effect>> = self
            .effects
            .iter()
            .map(|effect| match effect {
                EffectType::Equalizer(params) => {
                    let mut eq = Equalizer::new(sample_rate);
                    eq.set_params(*params);
                    Box::new(eq) as Box<dyn Effect>
                }
                EffectType::Compressor(params) => {
                    let mut comp = Compressor::new(sample_rate);
                    comp.set_params(*params, sample_rate);
                    Box::new(comp) as Box<dyn Effect>
                }
                EffectType::NoiseGate(params) => {
                    let mut gate = NoiseGate::new(sample_rate);
                    gate.set_params(*params, sample_rate);
                    Box::new(gate) as Box<dyn Effect>
                }
            })
            .collect();

        EffectsChainProcessor { effects: processors }
    }
}

impl Default for EffectsChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Runtime processor for effects chain
///
/// This holds the actual effect instances with their state.
/// Create from an `EffectsChain` configuration.
pub struct EffectsChainProcessor {
    effects: Vec<Box<dyn Effect>>,
}

impl EffectsChainProcessor {
    /// Process audio through all effects in the chain
    pub fn process(&mut self, buffer: &mut [f32]) -> Result<()> {
        for effect in &mut self.effects {
            if !effect.is_bypassed() {
                effect.process(buffer)?;
            }
        }
        Ok(())
    }

    /// Reset all effects in the chain
    pub fn reset(&mut self) {
        for effect in &mut self.effects {
            effect.reset();
        }
    }

    /// Set bypass state for an effect by index
    pub fn set_bypass(&mut self, index: usize, bypass: bool) -> bool {
        if let Some(effect) = self.effects.get_mut(index) {
            effect.set_bypass(bypass);
            true
        } else {
            false
        }
    }

    /// Get the number of effects
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: u32 = 48000;

    fn generate_test_signal(samples: usize, frequency: f32) -> Vec<f32> {
        (0..samples)
            .map(|i| (2.0 * std::f32::consts::PI * frequency * i as f32 / SAMPLE_RATE as f32).sin())
            .collect()
    }

    fn generate_silence(samples: usize) -> Vec<f32> {
        vec![0.0; samples]
    }

    // -------------------------------------------------------------------------
    // Biquad Filter Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_biquad_unity() {
        let coeffs = BiquadCoeffs::default();
        let mut filter = BiquadFilter::new(coeffs);

        let input = vec![0.5, 0.3, 0.7];
        let mut output = input.clone();

        filter.process(&mut output);

        // Unity gain should preserve signal
        for (in_sample, out_sample) in input.iter().zip(output.iter()) {
            assert!((in_sample - out_sample).abs() < 0.01);
        }
    }

    #[test]
    fn test_biquad_reset() {
        let coeffs = BiquadCoeffs::low_shelf(48000.0, 200.0, 6.0, 0.707);
        let mut filter = BiquadFilter::new(coeffs);

        // Process some signal
        let mut buffer = vec![0.5; 100];
        filter.process(&mut buffer);

        // Reset and process silence
        filter.reset();
        let mut silence = vec![0.0; 10];
        filter.process(&mut silence);

        // Output should be near zero after reset
        assert!(silence.iter().all(|&s| s.abs() < 0.01));
    }

    // -------------------------------------------------------------------------
    // Equalizer Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_equalizer_creation() {
        let eq = Equalizer::new(SAMPLE_RATE);
        assert!(!eq.is_bypassed());
        assert_eq!(eq.name(), "Equalizer");
        assert_eq!(eq.params().low_gain_db, 0.0);
        assert_eq!(eq.params().mid_gain_db, 0.0);
        assert_eq!(eq.params().high_gain_db, 0.0);
    }

    #[test]
    fn test_equalizer_bypass() {
        let mut eq = Equalizer::new(SAMPLE_RATE);

        assert!(!eq.is_bypassed());

        eq.set_bypass(true);
        assert!(eq.is_bypassed());

        let mut signal = vec![0.5, 0.3, 0.7];
        let original = signal.clone();

        eq.process(&mut signal).unwrap();

        // Bypassed signal should be unchanged
        assert_eq!(signal, original);
    }

    #[test]
    fn test_equalizer_gain_clamping() {
        let mut eq = Equalizer::new(SAMPLE_RATE);

        // Test clamping
        eq.set_low_gain(20.0);  // Should clamp to 12.0
        assert_eq!(eq.params().low_gain_db, 12.0);

        eq.set_low_gain(-20.0); // Should clamp to -12.0
        assert_eq!(eq.params().low_gain_db, -12.0);
    }

    #[test]
    fn test_equalizer_process() {
        let mut eq = Equalizer::new(SAMPLE_RATE);

        // Boost low shelf
        eq.set_low_gain(6.0);

        let low_freq = 100.0;
        let mut signal = generate_test_signal(1024, low_freq);
        let original_peak = signal.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);

        eq.process(&mut signal).unwrap();

        let processed_peak = signal.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);

        // Low frequencies should be boosted
        assert!(processed_peak > original_peak * 1.5);
    }

    #[test]
    fn test_equalizer_reset() {
        let mut eq = Equalizer::new(SAMPLE_RATE);
        eq.set_low_gain(6.0);

        let mut signal = vec![0.5; 100];
        eq.process(&mut signal).unwrap();

        eq.reset();

        // After reset, processing should not cause artifacts
        let mut silence = vec![0.0; 10];
        eq.process(&mut silence).unwrap();

        assert!(silence.iter().all(|&s| s.abs() < 0.01));
    }

    #[test]
    fn test_equalizer_params() {
        let mut eq = Equalizer::new(SAMPLE_RATE);

        let params = EqualizerParams {
            low_gain_db: 3.0,
            mid_freq: 800.0,
            mid_gain_db: -2.0,
            mid_q: 0.7,
            high_gain_db: 4.0,
        };

        eq.set_params(params);

        let actual = eq.params();
        assert_eq!(actual.low_gain_db, 3.0);
        assert_eq!(actual.mid_freq, 800.0);
        assert_eq!(actual.mid_gain_db, -2.0);
        assert_eq!(actual.mid_q, 0.7);
        assert_eq!(actual.high_gain_db, 4.0);
    }

    // -------------------------------------------------------------------------
    // Compressor Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_compressor_creation() {
        let comp = Compressor::new(SAMPLE_RATE);
        assert!(!comp.is_bypassed());
        assert_eq!(comp.name(), "Compressor");
        assert_eq!(comp.params().threshold_db, -18.0);
        assert_eq!(comp.params().ratio, 4.0);
    }

    #[test]
    fn test_compressor_bypass() {
        let mut comp = Compressor::new(SAMPLE_RATE);
        comp.set_bypass(true);

        let mut signal = vec![0.8, 0.6, 0.9];
        let original = signal.clone();

        comp.process(&mut signal).unwrap();

        assert_eq!(signal, original);
    }

    #[test]
    fn test_compressor_reduction() {
        let mut comp = Compressor::new(SAMPLE_RATE);
        comp.set_threshold(-10.0, SAMPLE_RATE);
        comp.set_ratio(4.0);
        comp.set_attack(0.001, SAMPLE_RATE);
        comp.set_release(0.01, SAMPLE_RATE);

        // Create a signal above threshold
        let mut signal = vec![0.8; 1024]; // ~ -2dB, well above -10dB threshold
        let original_rms = signal.iter().map(|s| s * s).sum::<f32>().sqrt() / signal.len() as f32;

        comp.process(&mut signal).unwrap();

        let processed_rms = signal.iter().map(|s| s * s).sum::<f32>().sqrt() / signal.len() as f32;

        // Signal should be reduced
        assert!(processed_rms < original_rms);
    }

    #[test]
    fn test_compressor_makeup_gain() {
        let mut comp = Compressor::new(SAMPLE_RATE);
        comp.set_threshold(-20.0, SAMPLE_RATE);
        comp.set_ratio(4.0);
        comp.set_makeup_gain(6.0);

        let mut signal = vec![0.5; 1024];
        comp.process(&mut signal).unwrap();

        // With makeup gain, signal should be amplified
        assert!(signal[0] > 0.5);
    }

    #[test]
    fn test_compressor_param_limits() {
        let mut comp = Compressor::new(SAMPLE_RATE);

        comp.set_threshold(-100.0, SAMPLE_RATE);
        assert_eq!(comp.params().threshold_db, params::DB_MIN);

        comp.set_ratio(50.0);
        assert_eq!(comp.params().ratio, params::RATIO_MAX);

        comp.set_makeup_gain(100.0);
        assert_eq!(comp.params().makeup_gain_db, params::DB_MAX);
    }

    #[test]
    fn test_compressor_reset() {
        let mut comp = Compressor::new(SAMPLE_RATE);
        comp.set_threshold(-10.0, SAMPLE_RATE);

        let mut signal = vec![0.8; 100];
        comp.process(&mut signal).unwrap();

        comp.reset();

        // After reset, envelope should start fresh
        let mut quiet_signal = vec![0.01; 10];
        comp.process(&mut quiet_signal).unwrap();

        // Quiet signal should not be heavily compressed after reset
        assert!(quiet_signal[0] > 0.005);
    }

    // -------------------------------------------------------------------------
    // Noise Gate Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_noise_gate_creation() {
        let gate = NoiseGate::new(SAMPLE_RATE);
        assert!(!gate.is_bypassed());
        assert_eq!(gate.name(), "NoiseGate");
        assert_eq!(gate.params().threshold_db, -40.0);
    }

    #[test]
    fn test_noise_gate_below_threshold() {
        let mut gate = NoiseGate::new(SAMPLE_RATE);
        gate.set_threshold(-20.0, SAMPLE_RATE);
        gate.set_attack(0.001, SAMPLE_RATE);
        gate.set_release(0.01, SAMPLE_RATE);

        // Signal below threshold (very quiet)
        let mut signal = vec![0.001; 1024];
        gate.process(&mut signal).unwrap();

        // Should be gated (near zero)
        assert!(signal.iter().all(|&s| s.abs() < 0.0005));
    }

    #[test]
    fn test_noise_gate_above_threshold() {
        let mut gate = NoiseGate::new(SAMPLE_RATE);
        gate.set_threshold(-40.0, SAMPLE_RATE);
        gate.set_attack(0.001, SAMPLE_RATE);
        gate.set_release(0.01, SAMPLE_RATE);

        // Signal above threshold - need enough samples for gate to open
        let mut signal = vec![0.5; 1024];
        gate.process(&mut signal).unwrap();

        // Check mid-buffer (after attack has had time to open the gate)
        let mid_point = signal.len() / 2;
        assert!(signal[mid_point] > 0.4);
    }

    #[test]
    fn test_noise_gate_sidechain() {
        let mut gate = NoiseGate::new(SAMPLE_RATE);
        gate.set_threshold(-20.0, SAMPLE_RATE);
        gate.set_sidechain(true);

        // Input signal is quiet
        let mut signal = vec![0.001; 1024];

        // But sidechain is loud
        gate.set_sidechain_level(-10.0);

        gate.process(&mut signal).unwrap();

        // Gate should open due to sidechain - check mid-buffer
        let mid_point = signal.len() / 2;
        assert!(signal[mid_point] > 0.0005);
    }

    #[test]
    fn test_noise_gate_hold() {
        let mut gate = NoiseGate::new(SAMPLE_RATE);
        gate.set_threshold(-20.0, SAMPLE_RATE);
        gate.set_hold(0.05); // 50ms hold - shorter for test
        gate.set_attack(0.001, SAMPLE_RATE);
        gate.set_release(0.01, SAMPLE_RATE);

        // Start with loud signal - enough to fully open gate
        let mut signal = vec![0.5; 1000];
        gate.process(&mut signal).unwrap();

        // Then go quiet - process enough samples for hold to expire
        let hold_samples = (0.06 * SAMPLE_RATE as f32) as usize; // Slightly more than hold
        let mut quiet = vec![0.001; hold_samples];
        gate.process(&mut quiet).unwrap();

        // Should close after hold expires - check near end
        let release_point = quiet.len() - 500;
        assert!(quiet[release_point] < 0.001);
    }

    // -------------------------------------------------------------------------
    // Effects Chain Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_effects_chain_empty() {
        let chain = EffectsChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn test_effects_chain_add_remove() {
        let mut chain = EffectsChain::new();

        chain.add(EffectType::Equalizer(EqualizerParams::default()));
        assert_eq!(chain.len(), 1);

        chain.add(EffectType::Compressor(CompressorParams::default()));
        assert_eq!(chain.len(), 2);

        chain.remove(0).unwrap();
        assert_eq!(chain.len(), 1);

        chain.clear();
        assert!(chain.is_empty());
    }

    #[test]
    fn test_effects_chain_processor() {
        let mut chain = EffectsChain::new();

        chain.add(EffectType::Equalizer(EqualizerParams {
            low_gain_db: 3.0,
            ..Default::default()
        }));
        chain.add(EffectType::Compressor(CompressorParams {
            threshold_db: -10.0,
            ..Default::default()
        }));

        let mut processor = chain.create_processor(SAMPLE_RATE);
        assert_eq!(processor.len(), 2);

        let mut signal = vec![0.5; 1024];
        processor.process(&mut signal).unwrap();

        // Signal should be modified by both effects
        assert_ne!(signal, vec![0.5; 1024]);
    }

    #[test]
    fn test_effects_chain_bypass() {
        let mut chain = EffectsChain::new();

        chain.add(EffectType::Equalizer(EqualizerParams::default()));
        chain.add(EffectType::Compressor(CompressorParams::default()));

        let mut processor = chain.create_processor(SAMPLE_RATE);

        // Bypass first effect
        processor.set_bypass(0, true);

        let mut signal = vec![0.5; 1024];
        processor.process(&mut signal).unwrap();

        // Signal should still be modified by compressor
        assert_ne!(signal, vec![0.5; 1024]);
    }

    #[test]
    fn test_effects_chain_reset() {
        let mut chain = EffectsChain::new();
        chain.add(EffectType::NoiseGate(NoiseGateParams::default()));

        let mut processor = chain.create_processor(SAMPLE_RATE);

        let mut signal = vec![0.5; 100];
        processor.process(&mut signal).unwrap();

        processor.reset();

        // After reset, state should be cleared
        let mut quiet = vec![0.0; 10];
        processor.process(&mut quiet).unwrap();

        assert!(quiet.iter().all(|&s| s.abs() < 0.01));
    }

    // -------------------------------------------------------------------------
    // Performance Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_performance_single_effect() {
        let mut eq = Equalizer::new(SAMPLE_RATE);

        let mut signal = generate_test_signal(48000, 440.0); // 1 second at 48kHz

        let start = std::time::Instant::now();
        for _ in 0..10 {
            eq.process(&mut signal).unwrap();
        }
        let duration = start.elapsed();

        // Should process 10 seconds in under 100ms (< 1% CPU at 10x real-time)
        assert!(duration.as_millis() < 100);
    }

    #[test]
    fn test_performance_full_chain() {
        let mut chain = EffectsChain::new();
        chain.add(EffectType::Equalizer(EqualizerParams::default()));
        chain.add(EffectType::Compressor(CompressorParams::default()));
        chain.add(EffectType::NoiseGate(NoiseGateParams::default()));

        let mut processor = chain.create_processor(SAMPLE_RATE);

        let mut signal = generate_test_signal(48000, 440.0);

        let start = std::time::Instant::now();
        for _ in 0..10 {
            processor.process(&mut signal).unwrap();
        }
        let duration = start.elapsed();

        // Should process 10 seconds in under 200ms with all 3 effects
        assert!(duration.as_millis() < 200);
    }
}
