package com.securepass.app

import android.app.Activity
import android.content.ClipData
import android.content.ClipDescription
import android.content.ClipboardManager
import android.content.Context
import android.os.Build
import android.os.PersistableBundle
import androidx.core.view.WindowCompat
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

@InvokeArg
class SystemBarsArgs {
    var dark: Boolean = false
}

@InvokeArg
class CopySensitiveArgs {
    lateinit var text: String
}

// Small Android-only helpers used from Rust (see android_native.rs).
@TauriPlugin
class AndroidNativePlugin(private val activity: Activity) : Plugin(activity) {

    // The app draws edge-to-edge behind the status and navigation bars, so
    // their icon colour must follow the app's theme (which can differ from the
    // system's when the user picks Light or Dark in Settings).
    @Command
    fun setSystemBarsDark(invoke: Invoke) {
        val args = invoke.parseArgs(SystemBarsArgs::class.java)
        activity.runOnUiThread {
            val controller = WindowCompat.getInsetsController(activity.window, activity.window.decorView)
            // "Light" bars = dark icons, for a light app background.
            controller.isAppearanceLightStatusBars = !args.dark
            controller.isAppearanceLightNavigationBars = !args.dark
            invoke.resolve()
        }
    }

    // Copies a secret marked as sensitive, so Android 13+ shows dots instead
    // of the value in its clipboard preview and keyboards don't suggest it.
    @Command
    fun copySensitive(invoke: Invoke) {
        val args = invoke.parseArgs(CopySensitiveArgs::class.java)
        activity.runOnUiThread {
            val clip = ClipData.newPlainText("SecurePass", args.text)
            clip.description.extras = PersistableBundle().apply {
                // ClipDescription.EXTRA_IS_SENSITIVE on API 33+; the same key
                // string is honoured by some older OEM builds and is harmless.
                val key = if (Build.VERSION.SDK_INT >= 33) ClipDescription.EXTRA_IS_SENSITIVE
                          else "android.content.extra.IS_SENSITIVE"
                putBoolean(key, true)
            }
            clipboard().setPrimaryClip(clip)
            invoke.resolve()
        }
    }

    @Command
    fun clearClipboard(invoke: Invoke) {
        activity.runOnUiThread {
            val cm = clipboard()
            if (Build.VERSION.SDK_INT >= 28) cm.clearPrimaryClip()
            else cm.setPrimaryClip(ClipData.newPlainText("", ""))
            invoke.resolve()
        }
    }

    private fun clipboard(): ClipboardManager =
        activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
}
