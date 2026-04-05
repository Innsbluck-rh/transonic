package com.innsb.transonic

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import androidx.activity.enableEdgeToEdge
import androidx.core.content.ContextCompat
import com.innsb.transonic.playback.PlaybackService
import com.innsb.transonic.playback.RustPlaybackBridge

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    if (
      Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
        ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
    ) {
      requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 1001)
    }

    if (intent?.getBooleanExtra(PlaybackService.EXTRA_FROM_NOTIFICATION, false) == true) {
      RustPlaybackBridge().notifyAppResumedFromNotification()
    }
  }

  override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    if (intent.getBooleanExtra(PlaybackService.EXTRA_FROM_NOTIFICATION, false)) {
      RustPlaybackBridge().notifyAppResumedFromNotification()
    }
  }
}