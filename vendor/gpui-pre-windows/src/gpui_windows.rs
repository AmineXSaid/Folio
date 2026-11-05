#![cfg(target_os = "windows")]

mod clipboard;
mod destination_list;
mod dialog;
mod direct_manipulation;
mod direct_write;
mod directx_atlas;
mod directx_devices;
mod directx_renderer;
mod dispatcher;
mod display;
mod events;
mod keyboard;
mod platform;
mod system_notifications;
mod system_settings;
mod util;
mod vsync;
mod window;
mod wrapper;

pub(crate) use clipboard::*;
pub(crate) use destination_list::*;
pub(crate) use direct_write::*;
pub(crate) use directx_atlas::*;
pub(crate) use directx_devices::*;
pub(crate) use directx_renderer::*;
pub(crate) use dispatcher::*;
pub(crate) use display::*;
pub(crate) use events::*;
pub(crate) use keyboard::*;
pub(crate) use platform::*;
pub(crate) use system_notifications::*;
pub(crate) use system_settings::*;
pub(crate) use util::*;
pub(crate) use vsync::*;
pub(crate) use window::*;
pub(crate) use wrapper::*;

pub use platform::WindowsPlatform;

/// How hard a pen pressed at its last move or touch, from 0 to 1, read
/// from the pointer messages before they're passed on as mouse moves; None
/// while the pointer isn't a pen. Folio's ink reads it (`FOLIO-CHANGES.md`).
pub fn pen_pressure() -> Option<f32> {
    let p = f32::from_bits(PEN_PRESSURE.load(std::sync::atomic::Ordering::Relaxed));
    (p >= 0.).then_some(p)
}

/// The last pen pressure, as `f32` bits; negative for none.
pub(crate) static PEN_PRESSURE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0xbf80_0000);

pub(crate) use windows::Win32::Foundation::HWND;
