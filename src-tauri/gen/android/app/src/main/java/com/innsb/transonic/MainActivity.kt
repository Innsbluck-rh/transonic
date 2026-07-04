package com.innsb.transonic

import android.content.Intent
import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.view.WindowManager
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.innsb.transonic.playback.PlaybackService
import com.innsb.transonic.playback.RustPlaybackBridge
import kotlin.math.roundToInt

class MainActivity : TauriActivity() {
  private var mainWebView: WebView? = null
  private var safeAreaInsetTop = "0px"
  private var safeAreaInsetRight = "0px"
  private var safeAreaInsetBottom = "0px"
  private var safeAreaInsetLeft = "0px"
  private var appliedStatusBarColor: Int? = null
  private var appliedNavigationBarColor: Int? = null
  private var appliedUseDarkSystemBarIcons = false

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
    webView.addJavascriptInterface(SystemBarsBridge(), "TransonicSystemBars")

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
    RustPlaybackBridge().notifyAppResumed()
    reapplySystemBarsTheme()
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

  private fun applySystemBarsTheme(statusBarColorText: String?, navigationBarColorText: String?, useDarkSystemBarIcons: Boolean) {
    val statusBarColor = parseCssColor(statusBarColorText, Color.BLACK)
    val requestedNavigationBarColor = parseCssColor(navigationBarColorText, statusBarColor)
    applySystemBarsTheme(statusBarColor, requestedNavigationBarColor, useDarkSystemBarIcons)
  }

  @Suppress("DEPRECATION")
  private fun applySystemBarsTheme(statusBarColor: Int, requestedNavigationBarColor: Int, useDarkSystemBarIcons: Boolean) {
    val navigationBarSupportsDarkIcons = Build.VERSION.SDK_INT >= Build.VERSION_CODES.O
    val navigationBarColor =
      if (navigationBarSupportsDarkIcons || !useDarkSystemBarIcons) {
        requestedNavigationBarColor
      } else {
        Color.BLACK
      }

    window.clearFlags(WindowManager.LayoutParams.FLAG_TRANSLUCENT_STATUS or WindowManager.LayoutParams.FLAG_TRANSLUCENT_NAVIGATION)
    window.addFlags(WindowManager.LayoutParams.FLAG_DRAWS_SYSTEM_BAR_BACKGROUNDS)
    window.statusBarColor = statusBarColor
    window.navigationBarColor = navigationBarColor

    WindowInsetsControllerCompat(window, window.decorView).run {
      isAppearanceLightStatusBars = useDarkSystemBarIcons
      isAppearanceLightNavigationBars = navigationBarSupportsDarkIcons && useDarkSystemBarIcons
    }

    appliedStatusBarColor = statusBarColor
    appliedNavigationBarColor = requestedNavigationBarColor
    appliedUseDarkSystemBarIcons = useDarkSystemBarIcons
  }

  private fun reapplySystemBarsTheme() {
    val statusBarColor = appliedStatusBarColor ?: return
    applySystemBarsTheme(statusBarColor, appliedNavigationBarColor ?: statusBarColor, appliedUseDarkSystemBarIcons)
  }

  private fun parseCssColor(value: String?, fallback: Int): Int {
    val colorText = value?.trim().takeUnless { it.isNullOrEmpty() } ?: return fallback
    return runCatching { Color.parseColor(colorText) }.getOrDefault(fallback)
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

  private inner class SystemBarsBridge {
    @JavascriptInterface
    fun applyTheme(statusBarColor: String?, navigationBarColor: String?, useDarkSystemBarIcons: Boolean) {
      runOnUiThread {
        applySystemBarsTheme(statusBarColor, navigationBarColor, useDarkSystemBarIcons)
      }
    }
  }
}
