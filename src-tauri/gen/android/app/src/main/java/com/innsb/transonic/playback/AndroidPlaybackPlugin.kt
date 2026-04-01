package com.innsb.transonic.playback

import android.app.Activity
import android.content.Context
import android.content.Intent
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

@InvokeArg
class AndroidPlaybackHeader {
  lateinit var name: String
  lateinit var value: String
}

@InvokeArg
class LoadPreparedMediaArgs {
  lateinit var mediaId: String
  lateinit var streamUrl: String
  var headers: List<AndroidPlaybackHeader> = emptyList()
  var absoluteStartPositionMs: Long = 0
  var localStartPositionMs: Long = 0
  var autoplay: Boolean = false
  lateinit var title: String
  var artist: String? = null
  var album: String? = null
  var artworkUrl: String? = null
}

@InvokeArg
class SeekToArgs {
  var positionMs: Long = 0
}

data class CurrentPositionResponse(val positionMs: Long)

internal object PlaybackServiceHost {
  private var service: PlaybackService? = null
  private val pendingCallbacks = mutableListOf<(PlaybackService) -> Unit>()

  @Synchronized
  fun publish(service: PlaybackService) {
    this.service = service
    val callbacks = pendingCallbacks.toList()
    pendingCallbacks.clear()
    callbacks.forEach { callback -> callback(service) }
  }

  @Synchronized
  fun clear(service: PlaybackService) {
    if (this.service === service) {
      this.service = null
    }
  }

  fun withExistingService(block: (PlaybackService) -> Unit): Boolean {
    val existingService = synchronized(this) { service }
    if (existingService == null) {
      return false
    }

    block(existingService)
    return true
  }

  fun withService(context: Context, block: (PlaybackService) -> Unit) {
    val existingService = synchronized(this) {
      val current = service
      if (current == null) {
        pendingCallbacks.add(block)
      }
      current
    }
    if (existingService != null) {
      block(existingService)
      return
    }

    context.startService(Intent(context, PlaybackService::class.java))
  }
}

@TauriPlugin
class AndroidPlaybackPlugin(private val activity: Activity) : Plugin(activity) {
  @Command
  fun loadPreparedMedia(invoke: Invoke) {
    val args = invoke.parseArgs(LoadPreparedMediaArgs::class.java)
    PlaybackServiceHost.withService(activity.applicationContext) { service ->
      service.loadPreparedMedia(args) { result ->
        result.onSuccess {
          invoke.resolve()
        }.onFailure { error ->
          invoke.reject(error.message ?: "Failed to load Android playback media.")
        }
      }
    }
  }

  @Command
  fun pause(invoke: Invoke) {
    if (!PlaybackServiceHost.withExistingService { service ->
      service.pause()
      invoke.resolve()
    }) {
      invoke.resolve()
    }
  }

  @Command
  fun stop(invoke: Invoke) {
    if (!PlaybackServiceHost.withExistingService { service ->
      service.stopPlayback()
      invoke.resolve()
    }) {
      invoke.resolve()
    }
  }

  @Command
  fun seekTo(invoke: Invoke) {
    val args = invoke.parseArgs(SeekToArgs::class.java)
    if (!PlaybackServiceHost.withExistingService { service ->
      service.seekToAbsolutePosition(args.positionMs)
      invoke.resolve()
    }) {
      invoke.resolve()
    }
  }

  @Command
  fun currentPositionMs(invoke: Invoke) {
    if (!PlaybackServiceHost.withExistingService { service ->
      invoke.resolveObject(CurrentPositionResponse(service.currentAbsolutePositionMs()))
    }) {
      invoke.resolveObject(CurrentPositionResponse(0))
    }
  }
}
