package com.securepass.app

import android.content.res.Configuration
import android.os.Build
import android.os.Bundle
import android.view.View
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    // The app draws behind the status/nav bars, so their icon colour must
    // match the app theme. Start from the system day/night setting (the app's
    // default "System" theme); once the web UI loads it reports the theme it
    // actually applied, including a Light/Dark override, via AndroidNativePlugin.
    val night = (resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK) ==
      Configuration.UI_MODE_NIGHT_YES
    val insetsController = WindowCompat.getInsetsController(window, window.decorView)
    insetsController.isAppearanceLightStatusBars = !night
    insetsController.isAppearanceLightNavigationBars = !night

    // enableEdgeToEdge() draws content behind the on-screen keyboard, which
    // makes windowSoftInputMode="adjustResize" (in AndroidManifest.xml) a
    // no-op: the WebView never actually shrinks when the keyboard opens, so
    // its JS (innerHeight, visualViewport) never observes the keyboard
    // either, leaving a focused field permanently hidden behind it with no
    // way for the web layer to detect or correct that. Apply the IME inset
    // as bottom padding on the content view so the WebView genuinely
    // resizes, restoring adjustResize-equivalent behavior.
    // Keep Android's autofill service (Google Password Manager, etc.) out of
    // the whole app. Otherwise it reads the entry form and offers to save a
    // vault entry's title and password to the user's Google account.
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      window.decorView.importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
    }

    val content = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
      val imeBottom = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
      view.setPadding(view.paddingLeft, view.paddingTop, view.paddingRight, imeBottom)
      insets
    }
  }
}
