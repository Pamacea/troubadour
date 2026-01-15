//! Example demonstrating the configuration and preset management system
//!
//! Run with: cargo run --package troubadour-core --example config_demo

use troubadour_core::domain::config::{
    Command, CommandExecutor, CommandResult, PresetManager, TroubadourConfig,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("troubadour_core=debug,info")
        .init();

    println!("=== Troubadour Configuration Demo ===\n");

    // 1. Create factory default configuration
    println!("1. Creating factory default configuration...");
    let config = TroubadourConfig::factory_default();
    println!("   ✓ Created configuration with {} channels", config.mixer.channels.len());

    // 2. Save configuration to file
    println!("\n2. Saving configuration to file...");
    let config_path = "demo_config.toml";
    config.save_to_file(config_path).await?;
    println!("   ✓ Configuration saved to {}", config_path);

    // 3. Load configuration from file
    println!("\n3. Loading configuration from file...");
    let loaded_config = TroubadourConfig::load_from_file(config_path).await?;
    println!(
        "   ✓ Loaded configuration with {} channels",
        loaded_config.mixer.channels.len()
    );

    // 4. Display channel information
    println!("\n4. Channel configuration:");
    for (i, channel) in loaded_config.mixer.channels.iter().enumerate() {
        println!(
            "   {}. {} - Volume: {} dB, Muted: {}, Solo: {}",
            i + 1,
            channel.name,
            channel.volume_db,
            channel.muted,
            channel.solo
        );
    }

    // 5. Create mixer engine from configuration
    println!("\n5. Creating mixer engine from configuration...");
    let mixer_engine = loaded_config.mixer.to_mixer_engine();
    println!(
        "   ✓ Mixer engine created with {} channels",
        mixer_engine.channels().count()
    );

    // 6. Convert mixer engine back to configuration
    println!("\n6. Converting mixer engine back to configuration...");
    let config_from_engine = troubadour_core::domain::config::MixerConfig::from_mixer_engine(&mixer_engine);
    println!(
        "   ✓ Converted {} channels back to configuration",
        config_from_engine.channels.len()
    );

    // 7. Preset management
    println!("\n7. Preset management:");
    let preset_dir = std::path::PathBuf::from("demo_presets");
    let preset_manager = PresetManager::new(preset_dir.clone());

    // Save a preset
    println!("   Saving preset 'my_preset'...");
    preset_manager
        .save_preset("my_preset", &loaded_config)
        .await?;
    println!("   ✓ Preset saved");

    // List presets
    println!("   Listing available presets...");
    let presets = preset_manager.list_presets().await?;
    for preset in &presets {
        println!("   - {}", preset);
    }

    // Load a preset
    println!("   Loading preset 'my_preset'...");
    let loaded_preset = preset_manager.load_preset("my_preset").await?;
    println!(
        "   ✓ Loaded preset with {} channels",
        loaded_preset.mixer.channels.len()
    );

    // 8. Create a custom command executor
    println!("\n8. Command execution:");
    let executor = DemoCommandExecutor::new();

    // Execute some commands
    let commands = vec![
        Command::SetVolume {
            channel_id: "mic".to_string(),
            volume_db: -6.0,
        },
        Command::ToggleMute {
            channel_id: "music".to_string(),
        },
        Command::AddChannel {
            id: "new_channel".to_string(),
            name: "New Channel".to_string(),
        },
    ];

    for cmd in commands {
        let result = executor.execute(cmd).await;
        println!("   Command result: {:?}", result);
    }

    println!("\n=== Demo Complete ===");

    // Cleanup
    std::fs::remove_file(config_path)?;
    std::fs::remove_dir_all(preset_dir)?;

    Ok(())
}

/// Demo command executor implementation
struct DemoCommandExecutor;

impl DemoCommandExecutor {
    fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl CommandExecutor for DemoCommandExecutor {
    async fn execute(&self, command: Command) -> CommandResult {
        match command {
            Command::SetVolume {
                channel_id,
                volume_db,
            } => {
                println!(
                    "   → Setting volume of '{}' to {} dB",
                    channel_id, volume_db
                );
                CommandResult::VolumeChanged {
                    channel_id,
                    new_volume_db: volume_db,
                }
            }
            Command::ToggleMute { channel_id } => {
                println!("   → Toggling mute for '{}'", channel_id);
                CommandResult::MuteToggled {
                    channel_id,
                    muted: true,
                }
            }
            Command::AddChannel { id, name } => {
                println!("   → Adding channel '{}' ({})", name, id);
                CommandResult::ChannelAdded { id }
            }
            _ => CommandResult::Error("Command not implemented in demo".to_string()),
        }
    }
}
