use serde::{Deserialize, Serialize};

/// Configuration sérialisable d'un noise gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseGateConfig {
    pub threshold: f32,
    pub attack: f32,
    pub release: f32,
    pub enabled: bool,
}

impl Default for NoiseGateConfig {
    fn default() -> Self {
        Self {
            threshold: 0.005,
            attack: 0.3,
            release: 0.002,
            enabled: false, // Off par defaut
        }
    }
}

/// Configuration sérialisable d'un compresseur.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressorConfig {
    pub threshold: f32,
    pub ratio: f32,
    pub attack: f32,
    pub release: f32,
    pub makeup_gain: f32,
    pub enabled: bool,
}

impl Default for CompressorConfig {
    fn default() -> Self {
        Self {
            threshold: 0.4,
            ratio: 3.0,
            attack: 0.005,
            release: 0.02,
            makeup_gain: 1.2,
            enabled: true,
        }
    }
}

/// Configuration sérialisable d'une bande EQ.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqBandConfig {
    pub filter_type: String, // "low_shelf", "peaking", "high_shelf"
    pub frequency: f32,
    pub gain_db: f32,
    pub q: f32,
    pub enabled: bool,
}

/// Configuration sérialisable d'un EQ paramétrique.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqConfig {
    pub bands: Vec<EqBandConfig>,
    pub enabled: bool,
}

impl Default for EqConfig {
    fn default() -> Self {
        Self {
            bands: vec![
                EqBandConfig {
                    filter_type: "low_shelf".to_string(),
                    frequency: 200.0,
                    gain_db: 0.0,
                    q: 0.7,
                    enabled: true,
                },
                EqBandConfig {
                    filter_type: "peaking".to_string(),
                    frequency: 1000.0,
                    gain_db: 0.0,
                    q: 1.0,
                    enabled: true,
                },
                EqBandConfig {
                    filter_type: "high_shelf".to_string(),
                    frequency: 8000.0,
                    gain_db: 0.0,
                    q: 0.7,
                    enabled: true,
                },
            ],
            enabled: true,
        }
    }
}

/// Configuration sérialisable d'un limiter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimiterConfig {
    pub ceiling: f32,
    pub release: f32,
    pub enabled: bool,
}

impl Default for LimiterConfig {
    fn default() -> Self {
        Self {
            ceiling: 0.95,
            release: 0.01,
            enabled: true,
        }
    }
}

/// Preset complet d'une chaîne d'effets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectsPreset {
    pub name: String,
    pub noise_gate: NoiseGateConfig,
    pub eq: EqConfig,
    pub compressor: CompressorConfig,
    pub limiter: LimiterConfig,
}

impl EffectsPreset {
    /// Preset par défaut : DSP doux, gate off.
    pub fn default_preset() -> Self {
        Self {
            name: "Default".to_string(),
            noise_gate: NoiseGateConfig::default(),
            eq: EqConfig::default(),
            compressor: CompressorConfig::default(),
            limiter: LimiterConfig::default(),
        }
    }

    /// Preset pour le streaming : gate actif, compression plus forte.
    pub fn streaming() -> Self {
        Self {
            name: "Streaming".to_string(),
            noise_gate: NoiseGateConfig {
                threshold: 0.008,
                attack: 0.3,
                release: 0.003,
                enabled: true,
            },
            eq: EqConfig {
                bands: vec![
                    EqBandConfig {
                        filter_type: "low_shelf".to_string(),
                        frequency: 100.0,
                        gain_db: -3.0, // Couper les basses (rumble)
                        q: 0.7,
                        enabled: true,
                    },
                    EqBandConfig {
                        filter_type: "peaking".to_string(),
                        frequency: 3000.0,
                        gain_db: 2.0, // Boost presence
                        q: 1.0,
                        enabled: true,
                    },
                    EqBandConfig {
                        filter_type: "high_shelf".to_string(),
                        frequency: 10000.0,
                        gain_db: 1.0, // Un peu d'air
                        q: 0.7,
                        enabled: true,
                    },
                ],
                enabled: true,
            },
            compressor: CompressorConfig {
                threshold: 0.25,
                ratio: 5.0,
                attack: 0.005,
                release: 0.03,
                makeup_gain: 1.5,
                enabled: true,
            },
            limiter: LimiterConfig::default(),
        }
    }

    /// Preset clean : pas de traitement, juste le limiter.
    pub fn clean() -> Self {
        Self {
            name: "Clean".to_string(),
            noise_gate: NoiseGateConfig {
                enabled: false,
                ..NoiseGateConfig::default()
            },
            eq: EqConfig {
                enabled: false,
                ..EqConfig::default()
            },
            compressor: CompressorConfig {
                enabled: false,
                ..CompressorConfig::default()
            },
            limiter: LimiterConfig::default(),
        }
    }

    /// Retourne tous les presets intégrés.
    pub fn builtin_presets() -> Vec<Self> {
        vec![Self::default_preset(), Self::streaming(), Self::clean()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preset() {
        let preset = EffectsPreset::default_preset();
        assert_eq!(preset.name, "Default");
        assert!(!preset.noise_gate.enabled);
        assert!(preset.compressor.enabled);
        assert!(preset.limiter.enabled);
    }

    #[test]
    fn streaming_preset() {
        let preset = EffectsPreset::streaming();
        assert_eq!(preset.name, "Streaming");
        assert!(preset.noise_gate.enabled);
        assert_eq!(preset.compressor.ratio, 5.0);
    }

    #[test]
    fn clean_preset() {
        let preset = EffectsPreset::clean();
        assert!(!preset.noise_gate.enabled);
        assert!(!preset.eq.enabled);
        assert!(!preset.compressor.enabled);
        assert!(preset.limiter.enabled); // Limiter always on
    }

    #[test]
    fn builtin_presets_count() {
        let presets = EffectsPreset::builtin_presets();
        assert_eq!(presets.len(), 3);
    }

    #[test]
    fn preset_serialization_roundtrip() {
        let preset = EffectsPreset::streaming();
        let toml_str = toml::to_string_pretty(&preset).unwrap();
        let parsed: EffectsPreset = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.name, "Streaming");
        assert_eq!(parsed.eq.bands.len(), 3);
    }
}
