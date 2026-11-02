//! VST3 hosting. Loading, controllers, and state stay on the TUI thread;
//! a dedicated worker processes audio and the device callback only reads samples.
mod catalog;
mod rack;
mod service;
pub(crate) type SharedPlugin = std::sync::Arc<std::sync::Mutex<vst3_host::Plugin>>;

pub(crate) fn lock_plugin(
    plugin: &SharedPlugin,
) -> anyhow::Result<std::sync::MutexGuard<'_, vst3_host::Plugin>> {
    plugin
        .lock()
        .map_err(|_| anyhow::anyhow!("VST3 plugin lock poisoned"))
}
#[cfg(test)]
mod soundfont_tests;
#[cfg(test)]
mod vst_tests;
pub use catalog::{SynthConfig, SynthKind, available};
pub use service::{Client, ParameterView};

#[cfg(test)]
pub(crate) fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    // Native module entry/exit and factory caches are process-wide. Integration
    // tests must not load/unload the same bundle from unrelated control threads.
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|poison| poison.into_inner())
}
