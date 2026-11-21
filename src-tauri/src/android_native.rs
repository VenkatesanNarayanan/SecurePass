// Android-only helpers implemented in Kotlin
// (gen/android/app/src/main/java/com/securepass/app/AndroidNativePlugin.kt):
//
// - System bar icon colour. The activity draws edge-to-edge (MainActivity.kt),
//   so the app's background shows behind the status/navigation bars; with the
//   wrong icon colour the clock, signal and battery icons become invisible.
//   The frontend calls `set_system_bars_theme` whenever the theme changes.
// - Clipboard. Copies are marked sensitive so Android 13+ hides the value in
//   its clipboard preview, and clearing uses `clearPrimaryClip()` rather than
//   writing an empty string.
use tauri::{
    plugin::{Builder, PluginHandle, TauriPlugin},
    Manager, Runtime,
};

const PLUGIN_IDENTIFIER: &str = "com.securepass.app";

pub struct AndroidNative<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> AndroidNative<R> {
    fn run(&self, command: &str, args: serde_json::Value) -> Result<(), String> {
        self.0
            .run_mobile_plugin::<()>(command, args)
            .map_err(|e| e.to_string())
    }

    pub fn set_system_bars_dark(&self, dark: bool) -> Result<(), String> {
        self.run("setSystemBarsDark", serde_json::json!({ "dark": dark }))
    }

    pub fn copy_sensitive(&self, text: &str) -> Result<(), String> {
        self.run("copySensitive", serde_json::json!({ "text": text }))
    }

    pub fn clear_clipboard(&self) -> Result<(), String> {
        self.run("clearClipboard", serde_json::json!({}))
    }
}

pub trait AndroidNativeExt<R: Runtime> {
    fn android_native(&self) -> &AndroidNative<R>;
}

impl<R: Runtime, T: Manager<R>> AndroidNativeExt<R> for T {
    fn android_native(&self) -> &AndroidNative<R> {
        self.state::<AndroidNative<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("android-native")
        .setup(|app, api| {
            let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, "AndroidNativePlugin")?;
            app.manage(AndroidNative(handle));
            Ok(())
        })
        .build()
}
