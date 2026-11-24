pub mod app;
pub mod audio;
pub mod badge;
pub mod bezel;
pub mod boot_times;
pub mod bootlogo;
pub mod build_info;
pub mod cable;
pub mod cart_art;
pub mod cart_faces;
pub mod core;
pub mod core_picker;
pub mod crash;
pub mod disc_picker;
pub mod drc;
pub mod emu;
pub mod face_builder;
pub mod fonts;
pub mod frames;
pub mod frontend;
pub mod input;
pub mod label_cache;
pub mod latency;
pub mod link_art_builder;
pub mod link_kind;
pub mod link_net;
pub mod link_radio;
pub mod link_screen;
pub mod link_start;
pub mod link_state;
pub mod menu_status;
pub mod net;
pub mod offline_ra;
pub mod overlay;
pub mod persist;
pub mod pip;
mod pip_art;
pub mod prefetch;
pub mod quiet;
pub mod ra;
pub mod resample;
pub mod rewind;
pub mod root;
pub mod session;
mod sheet;
pub mod sntp;
pub mod state_writer;
pub mod sync;
pub mod thumb;
pub mod update;
pub mod video_mode;
pub mod wallpaper;

#[cfg(feature = "brick")]
pub mod brick_led;

#[cfg(feature = "brick")]
pub mod brick_power;

#[cfg(feature = "brick")]
pub mod brick_wifi;

#[cfg(feature = "brick")]
pub mod brick_update;

#[cfg(feature = "brick")]
pub mod brick_link;

#[cfg(feature = "sim")]
pub mod sim;
