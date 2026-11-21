package com.securepass.app

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Intent
import androidx.core.net.toUri
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

@InvokeArg
class OpenInChromeArgs {
    lateinit var url: String
}

// Explicitly targets Chrome (as opposed to whatever the user's default
// browser is) for the OAuth flow — see chrome_opener.rs for why.
@TauriPlugin
class ChromeOpenerPlugin(private val activity: Activity) : Plugin(activity) {
    @Command
    fun openInChrome(invoke: Invoke) {
        val args = invoke.parseArgs(OpenInChromeArgs::class.java)
        try {
            val intent = Intent(Intent.ACTION_VIEW, args.url.toUri())
            intent.setPackage("com.android.chrome")
            intent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            activity.applicationContext.startActivity(intent)
            invoke.resolve()
        } catch (ex: ActivityNotFoundException) {
            invoke.reject("Chrome is not installed")
        }
    }
}
