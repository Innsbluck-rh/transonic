package com.innsb.transonic

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import com.innsb.transonic.playback.PlaybackService
import com.innsb.transonic.playback.RustPlaybackBridge
import kotlin.math.roundToInt

class MainActivity : TauriActivity() {
  private var mainWebView: WebView? = null
  private var safeAreaInsetTop = "0px"
  private var safeAreaInsetRight = "0px"
  private var safeAreaInsetBottom = "0px"
  private var safeAreaInsetLeft = "0px"

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    if (intent?.getBooleanExtra(PlaybackService.EXTRA_FROM_NOTIFICATION, false) == true) {
      RustPlaybackBridge().notifyAppResumedFromNotification()
    }
  }

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    mainWebView = webView
    webView.addJavascriptInterface(SafeAreaInsetsBridge(), "TransonicSafeAreaInsets")

    ViewCompat.setOnApplyWindowInsetsListener(webView) { _, windowInsets ->
      syncSafeAreaInsets(webView, windowInsets)
      windowInsets
    }

    webView.post {
      ViewCompat.requestApplyInsets(webView)
      syncSafeAreaInsets(webView, ViewCompat.getRootWindowInsets(webView))
    }
    webView.postDelayed({ syncSafeAreaInsets(webView, ViewCompat.getRootWindowInsets(webView)) }, 250L)
  }

  override fun onResume() {
    super.onResume()
    mainWebView?.let { webView ->
      ViewCompat.requestApplyInsets(webView)
      webView.post { syncSafeAreaInsets(webView, ViewCompat.getRootWindowInsets(webView)) }
    }
  }

  override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    if (intent.getBooleanExtra(PlaybackService.EXTRA_FROM_NOTIFICATION, false)) {
      RustPlaybackBridge().notifyAppResumedFromNotification()
    }
  }

  private fun syncSafeAreaInsets(webView: WebView, windowInsets: WindowInsetsCompat?) {
    val safeInsets =
      windowInsets?.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
        ?: return
    safeAreaInsetTop = toCssPx(webView, safeInsets.top)
    safeAreaInsetRight = toCssPx(webView, safeInsets.right)
    safeAreaInsetBottom = toCssPx(webView, safeInsets.bottom)
    safeAreaInsetLeft = toCssPx(webView, safeInsets.left)

    val script =
      """
      (function () {
        var top = '$safeAreaInsetTop';
        var right = '$safeAreaInsetRight';
        var bottom = '$safeAreaInsetBottom';
        var left = '$safeAreaInsetLeft';
        var root = document.documentElement;
        if (root) {
          root.style.setProperty('--safe-area-inset-top', top);
          root.style.setProperty('--safe-area-inset-right', right);
          root.style.setProperty('--safe-area-inset-bottom', bottom);
          root.style.setProperty('--safe-area-inset-left', left);
        }
        try {
          localStorage.setItem('transonic.safeAreaCssInsetTop', top);
          localStorage.setItem('transonic.safeAreaCssInsetRight', right);
          localStorage.setItem('transonic.safeAreaCssInsetBottom', bottom);
          localStorage.setItem('transonic.safeAreaCssInsetLeft', left);
        } catch (_) {}
      })();
      """.trimIndent()

    webView.evaluateJavascript(script, null)
  }

  private fun toCssPx(webView: WebView, physicalPx: Int): String {
    val density = webView.resources.displayMetrics.density.takeIf { it > 0f } ?: 1f
    return "${(physicalPx / density).roundToInt()}px"
  }

  private inner class SafeAreaInsetsBridge {
    @JavascriptInterface
    fun top(): String = safeAreaInsetTop

    @JavascriptInterface
    fun right(): String = safeAreaInsetRight

    @JavascriptInterface
    fun bottom(): String = safeAreaInsetBottom

    @JavascriptInterface
    fun left(): String = safeAreaInsetLeft
  }
}
