//! Helper utilities for benchmarks

/// Generate sine wave test signal
pub fn generate_sine_wave(freq: f32, sample_rate: u32, frames: usize) -> Vec<f32> {
    (0..frames)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * freq * t).sin()
        })
        .collect()
}

/// Generate white noise test signal
pub fn generate_white_noise(frames: usize) -> Vec<f32> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..frames).map(|_| rng.gen::<f32>() * 2.0 - 1.0).collect()
}

/// Generate silence
pub fn generate_silence(frames: usize) -> Vec<f32> {
    vec![0.0; frames]
}

/// Calculate RMS level
pub fn calc_rms(buffer: &[f32]) -> f32 {
    let sum_sq: f32 = buffer.iter().map(|&s| s * s).sum();
    (sum_sq / buffer.len() as f32).sqrt()
}

/// Calculate peak level
pub fn calc_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|&s| s.abs()).fold(0.0f32, f32::max)
}

/// Convert linear amplitude to decibels
pub fn amplitude_to_db(amp: f32) -> f32 {
    if amp <= 0.0 {
        -100.0
    } else {
        20.0 * amp.log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_sine_wave() {
        let wave = generate_sine_wave(440.0, 48000, 512);
        assert_eq!(wave.len(), 512);
        assert!(wave.iter().all(|&s| s >= -1.0 && s <= 1.0));
    }

    #[test]
    fn test_generate_white_noise() {
        let noise = generate_white_noise(512);
        assert_eq!(noise.len(), 512);
        assert!(noise.iter().all(|&s| s >= -1.0 && s <= 1.0));
    }

    #[test]
    fn test_calc_rms() {
        let signal = vec![1.0, -1.0, 1.0, -1.0];
        let rms = calc_rms(&signal);
        assert!((rms - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_calc_peak() {
        let signal = vec![0.5, -0.8, 0.3, -0.2];
        let peak = calc_peak(&signal);
        assert!((peak - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_amplitude_to_db() {
        assert!((amplitude_to_db(1.0) - 0.0).abs() < 0.1);
        assert!((amplitude_to_db(0.5) - (-6.02)).abs() < 0.1);
        assert!((amplitude_to_db(0.0) - (-100.0)).abs() < 0.1);
    }
}
