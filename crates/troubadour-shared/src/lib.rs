// `pub mod` déclare un module public.
// En Rust, chaque fichier est un module. `mod audio` cherche
// soit `audio.rs` soit `audio/mod.rs` dans le même dossier.
// `pub` le rend accessible depuis l'extérieur de la crate.
pub mod audio;
pub mod config;
pub mod dsp;
pub mod error;
pub mod messages;
pub mod mixer;
pub mod profile;
