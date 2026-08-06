package com.securepass.app

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

    // The app UI has no dark mode — it's always a white/light background.
    // enableEdgeToEdge() otherwise picks status/nav bar icon color from the
    // system's day/night setting, so a phone in dark mode gets white icons
    // on our white header and they vanish. Force dark (visible) icons always.
    val insetsController = WindowCompat.getInsetsController(window, window.decorView)
    insetsController.isAppearanceLightStatusBars = true
    insetsController.isAppearanceLightNavigationBars = true

    // enableEdgeToEdge() draws content behind the on-screen keyboard, which
    // makes windowSoftInputMode="adjustResize" (in AndroidManifest.xml) a
    // no-op: the WebView never actually shrinks when the keyboard opens, so
    // its JS (innerHeight, visualViewport) never observes the keyboard
    // either, leaving a focused field permanently hidden behind it with no
    // way for the web layer to detect or correct that. Apply the IME inset
    // as bottom padding on the content view so the WebView genuinely
    // resizes, restoring adjustResize-equivalent behavior.
    val content = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
      val imeBottom = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
      view.setPadding(view.paddingLeft, view.paddingTop, view.paddingRight, imeBottom)
      insets
    }
  }
}
