// Android-only helper that opens a URL specifically in Chrome, bypassing
// whatever the user's default browser is. Some OEM browsers (Samsung
// Internet observed in practice) fail to hand the OAuth redirect's custom
// URI scheme back to the app at all, silently dropping the user onto an
// unrelated screen instead. `tauri_plugin_opener`'s `open_url` always opens
// the default browser on mobile with no way to target a specific app, so
// this is a minimal Tauri Android plugin wrapping an explicit
// `Intent.setPackage("com.android.chrome")` in Kotlin (see
// `gen/android/app/src/main/java/com/securepass/app/ChromeOpenerPlugin.kt`).
use tauri::{
    plugin::{Builder, PluginHandle, TauriPlugin},
    Manager, Runtime,
};

const PLUGIN_IDENTIFIER: &str = "com.securepass.app";

pub struct ChromeOpener<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> ChromeOpener<R> {
    /// Opens `url` in Chrome specifically. Errs (e.g. Chrome not installed)
    /// so the caller can fall back to the default-browser opener.
    pub fn open_in_chrome(&self, url: &str) -> Result<(), String> {
        self.0
            .run_mobile_plugin::<()>("openInChrome", serde_json::json!({ "url": url }))
            .map_err(|e| e.to_string())
    }
}

pub trait ChromeOpenerExt<R: Runtime> {
    fn chrome_opener(&self) -> &ChromeOpener<R>;
}

impl<R: Runtime, T: Manager<R>> ChromeOpenerExt<R> for T {
    fn chrome_opener(&self) -> &ChromeOpener<R> {
        self.state::<ChromeOpener<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("chrome-opener")
        .setup(|app, api| {
            let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, "ChromeOpenerPlugin")?;
            app.manage(ChromeOpener(handle));
            Ok(())
        })
        .build()
}
