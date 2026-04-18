package com.innsb.transonic.playback

import android.app.Activity
import android.content.ComponentName
import android.content.Context
import android.os.Bundle
import androidx.annotation.Keep
import androidx.core.content.ContextCompat
import androidx.media3.session.MediaController
import androidx.media3.session.SessionCommand
import androidx.media3.session.SessionResult
import androidx.media3.session.SessionToken
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin
import com.google.common.util.concurrent.ListenableFuture

internal const val COMMAND_LOAD_PREPARED_MEDIA = "com.innsb.transonic.playback.LOAD_PREPARED_MEDIA"

private const val KEY_MEDIA_ID = "mediaId"
private const val KEY_STREAM_URL = "streamUrl"
private const val KEY_HEADER_NAMES = "headerNames"
private const val KEY_HEADER_VALUES = "headerValues"
private const val KEY_ABSOLUTE_START_POSITION_MS = "absoluteStartPositionMs"
private const val KEY_LOCAL_START_POSITION_MS = "localStartPositionMs"
private const val KEY_AUTOPLAY = "autoplay"
private const val KEY_TITLE = "title"
private const val KEY_ARTIST = "artist"
private const val KEY_ALBUM = "album"
private const val KEY_ARTWORK_PATH = "artworkPath"

@InvokeArg
@Keep
class AndroidPlaybackHeader {
  lateinit var name: String
  lateinit var value: String
}

@InvokeArg
@Keep
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
  var artworkPath: String? = null
}

@InvokeArg
@Keep
class SeekToArgs {
  var positionMs: Long = 0
}

@Keep
data class CurrentPositionResponse(val positionMs: Long)

private fun LoadPreparedMediaArgs.toBundle(): Bundle {
  val headerNames = ArrayList<String>(headers.size)
  val headerValues = ArrayList<String>(headers.size)
  headers.forEach { header ->
    headerNames.add(header.name)
    headerValues.add(header.value)
  }

  return Bundle().apply {
    putString(KEY_MEDIA_ID, mediaId)
    putString(KEY_STREAM_URL, streamUrl)
    putStringArrayList(KEY_HEADER_NAMES, headerNames)
    putStringArrayList(KEY_HEADER_VALUES, headerValues)
    putLong(KEY_ABSOLUTE_START_POSITION_MS, absoluteStartPositionMs)
    putLong(KEY_LOCAL_START_POSITION_MS, localStartPositionMs)
    putBoolean(KEY_AUTOPLAY, autoplay)
    putString(KEY_TITLE, title)
    putString(KEY_ARTIST, artist)
    putString(KEY_ALBUM, album)
    putString(KEY_ARTWORK_PATH, artworkPath)
  }
}

internal fun Bundle.toLoadPreparedMediaArgs(): LoadPreparedMediaArgs? {
  val mediaId = getString(KEY_MEDIA_ID) ?: return null
  val streamUrl = getString(KEY_STREAM_URL) ?: return null
  val title = getString(KEY_TITLE) ?: return null
  val headerNames = getStringArrayList(KEY_HEADER_NAMES) ?: arrayListOf()
  val headerValues = getStringArrayList(KEY_HEADER_VALUES) ?: arrayListOf()
  val headerCount = minOf(headerNames.size, headerValues.size)
  val headers = ArrayList<AndroidPlaybackHeader>(headerCount)

  repeat(headerCount) { index ->
    headers.add(
      AndroidPlaybackHeader().apply {
        name = headerNames[index]
        value = headerValues[index]
      },
    )
  }

  return LoadPreparedMediaArgs().apply {
    this.mediaId = mediaId
    this.streamUrl = streamUrl
    this.headers = headers
    this.absoluteStartPositionMs = getLong(KEY_ABSOLUTE_START_POSITION_MS)
    this.localStartPositionMs = getLong(KEY_LOCAL_START_POSITION_MS)
    this.autoplay = getBoolean(KEY_AUTOPLAY)
    this.title = title
    this.artist = getString(KEY_ARTIST)
    this.album = getString(KEY_ALBUM)
    this.artworkPath = getString(KEY_ARTWORK_PATH)
  }
}

private fun Throwable.userMessage(defaultMessage: String): String {
  var current: Throwable? = this
  while (current != null) {
    val message = current.message
    if (!message.isNullOrBlank()) {
      return message
    }
    current = current.cause
  }
  return defaultMessage
}

internal object PlaybackControllerHost {
  private var controller: MediaController? = null
  private var controllerFuture: ListenableFuture<MediaController>? = null
  private val pendingCallbacks = mutableListOf<(Result<MediaController>) -> Unit>()
  private var loadedStreamBasePositionMs: Long = 0

  fun withExistingController(block: (MediaController) -> Unit): Boolean {
    val existingController = synchronized(this) { controller }
    if (existingController == null) {
      return false
    }

    block(existingController)
    return true
  }

  fun withController(context: Context, block: (Result<MediaController>) -> Unit) {
    val executor = ContextCompat.getMainExecutor(context)
    val (existingController, shouldCreateFuture) = synchronized(this) {
      val currentController = controller
      if (currentController != null) {
        Pair(currentController, false)
      } else {
        pendingCallbacks.add(block)
        Pair(null, controllerFuture == null)
      }
    }

    if (existingController != null) {
      block(Result.success(existingController))
      return
    }

    if (!shouldCreateFuture) {
      return
    }

    val sessionToken = SessionToken(context, ComponentName(context, PlaybackService::class.java))
    val controllerFuture =
      MediaController.Builder(context, sessionToken)
        .setListener(
          object : MediaController.Listener {
            override fun onDisconnected(controller: MediaController) {
              clear(controller)
            }
          },
        )
        .buildAsync()

    synchronized(this) {
      this.controllerFuture = controllerFuture
    }

    controllerFuture.addListener(
      {
        val result =
          try {
            Result.success(controllerFuture.get())
          } catch (error: Throwable) {
            if (error is InterruptedException) {
              Thread.currentThread().interrupt()
            }
            Result.failure(error)
          }
        publish(result)
      },
      executor,
    )
  }

  fun updateLoadedStreamBasePositionMs(positionMs: Long) {
    synchronized(this) {
      loadedStreamBasePositionMs = positionMs
    }
  }

  fun loadedStreamBasePositionMs(): Long {
    return synchronized(this) { loadedStreamBasePositionMs }
  }

  fun releaseCurrentController() {
    releaseController(null)
  }

  @Synchronized
  private fun publish(result: Result<MediaController>) {
    controllerFuture = null
    controller = result.getOrNull()
    if (controller == null) {
      loadedStreamBasePositionMs = 0
    }

    val callbacks = pendingCallbacks.toList()
    pendingCallbacks.clear()
    callbacks.forEach { callback -> callback(result) }
  }

  @Synchronized
  private fun clear(controller: MediaController) {
    if (this.controller === controller) {
      this.controller = null
      loadedStreamBasePositionMs = 0
    }
  }

  private fun releaseController(controller: MediaController?) {
    val controllerToRelease =
      synchronized(this) {
        val currentController = this.controller
        if (controller != null && currentController !== controller) {
          null
        } else {
          this.controller = null
          loadedStreamBasePositionMs = 0
          currentController
        }
      }
    controllerToRelease?.release()
  }
}

@TauriPlugin
@Keep
class AndroidPlaybackPlugin(private val activity: Activity) : Plugin(activity) {
  @Command
  @Keep
  fun play(invoke: Invoke) {
    val applicationContext = activity.applicationContext
    PlaybackControllerHost.withController(applicationContext) { controllerResult ->
      controllerResult.onSuccess { controller ->
        controller.play()
        invoke.resolve()
      }.onFailure { error ->
        invoke.reject(error.userMessage("Failed to connect to Android playback controller."))
      }
    }
  }

  @Command
  @Keep
  fun loadPreparedMedia(invoke: Invoke) {
    val args = invoke.parseArgs(LoadPreparedMediaArgs::class.java)
    val applicationContext = activity.applicationContext
    val streamBasePositionMs =
      (args.absoluteStartPositionMs - args.localStartPositionMs).coerceAtLeast(0)

    PlaybackControllerHost.withController(applicationContext) { controllerResult ->
      controllerResult.onSuccess { controller ->
        val future =
          controller.sendCustomCommand(
            SessionCommand(COMMAND_LOAD_PREPARED_MEDIA, Bundle.EMPTY),
            args.toBundle(),
          )
        future.addListener(
          {
            try {
              val result = future.get()
              if (result.resultCode == SessionResult.RESULT_SUCCESS) {
                PlaybackControllerHost.updateLoadedStreamBasePositionMs(streamBasePositionMs)
                invoke.resolve()
              } else {
                invoke.reject("Failed to load Android playback media. resultCode=${result.resultCode}")
              }
            } catch (error: Throwable) {
              if (error is InterruptedException) {
                Thread.currentThread().interrupt()
              }
              invoke.reject(error.userMessage("Failed to load Android playback media."))
            }
          },
          ContextCompat.getMainExecutor(applicationContext),
        )
      }.onFailure { error ->
        invoke.reject(error.userMessage("Failed to connect to Android playback controller."))
      }
    }
  }

  @Command
  @Keep
  fun pause(invoke: Invoke) {
    if (!PlaybackControllerHost.withExistingController { controller ->
      controller.pause()
      invoke.resolve()
    }) {
      invoke.resolve()
    }
  }

  @Command
  @Keep
  fun stop(invoke: Invoke) {
    if (!PlaybackControllerHost.withExistingController { controller ->
      controller.stop()
      controller.clearMediaItems()
      PlaybackControllerHost.releaseCurrentController()
      invoke.resolve()
    }) {
      invoke.resolve()
    }
  }

  @Command
  @Keep
  fun seekTo(invoke: Invoke) {
    val args = invoke.parseArgs(SeekToArgs::class.java)
    if (!PlaybackControllerHost.withExistingController { controller ->
      val localPositionMs =
        (args.positionMs - PlaybackControllerHost.loadedStreamBasePositionMs()).coerceAtLeast(0)
      controller.seekTo(localPositionMs)
      invoke.resolve()
    }) {
      invoke.resolve()
    }
  }

  @Command
  @Keep
  fun currentPositionMs(invoke: Invoke) {
    if (!PlaybackControllerHost.withExistingController { controller ->
      val currentPositionMs = controller.currentPosition.coerceAtLeast(0)
      invoke.resolveObject(
        CurrentPositionResponse(
          PlaybackControllerHost.loadedStreamBasePositionMs() + currentPositionMs,
        ),
      )
    }) {
      invoke.resolveObject(CurrentPositionResponse(0))
    }
  }
}
