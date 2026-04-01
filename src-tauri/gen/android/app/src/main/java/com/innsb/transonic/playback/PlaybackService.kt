package com.innsb.transonic.playback

import android.app.PendingIntent
import android.content.Intent
import android.net.Uri
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.ForwardingPlayer
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.datasource.DefaultHttpDataSource
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.source.ProgressiveMediaSource
import androidx.media3.session.DefaultMediaNotificationProvider
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService
import com.innsb.transonic.MainActivity
import com.innsb.transonic.R

private data class PendingLoad(
  val mediaId: String,
  val callback: (Result<Unit>) -> Unit,
)

@UnstableApi
private class QueueCommandForwardingPlayer(
  player: Player,
  private val rustBridge: RustPlaybackBridge,
) : ForwardingPlayer(player) {
  override fun isCommandAvailable(command: Int): Boolean {
    return when (command) {
      Player.COMMAND_SEEK_TO_NEXT,
      Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM,
      Player.COMMAND_SEEK_TO_PREVIOUS,
      Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM -> true
      else -> super.isCommandAvailable(command)
    }
  }

  override fun getAvailableCommands(): Player.Commands {
    return super.getAvailableCommands()
      .buildUpon()
      .add(Player.COMMAND_SEEK_TO_NEXT)
      .add(Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM)
      .add(Player.COMMAND_SEEK_TO_PREVIOUS)
      .add(Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM)
      .build()
  }

  override fun hasNextMediaItem(): Boolean {
    return currentMediaItem != null
  }

  override fun hasPreviousMediaItem(): Boolean {
    return currentMediaItem != null
  }

  override fun getNextMediaItemIndex(): Int {
    return if (currentMediaItem != null) 0 else C.INDEX_UNSET
  }

  override fun getPreviousMediaItemIndex(): Int {
    return if (currentMediaItem != null) 0 else C.INDEX_UNSET
  }

  override fun seekToNext() {
    rustBridge.dispatchControllerAction("next")
  }

  override fun seekToNextMediaItem() {
    rustBridge.dispatchControllerAction("next")
  }

  override fun seekToPrevious() {
    rustBridge.dispatchControllerAction("prev")
  }

  override fun seekToPreviousMediaItem() {
    rustBridge.dispatchControllerAction("prev")
  }
}

@UnstableApi
class PlaybackService : MediaSessionService(), Player.Listener {
  private val rustBridge = RustPlaybackBridge()
  private lateinit var player: ExoPlayer
  private lateinit var sessionPlayer: QueueCommandForwardingPlayer
  private var mediaSession: MediaSession? = null
  private var pendingLoad: PendingLoad? = null
  private var loadedStreamBasePositionMs: Long = 0

  override fun onCreate() {
    super.onCreate()

    player = ExoPlayer.Builder(this).build().apply {
      setAudioAttributes(
        AudioAttributes.Builder()
          .setUsage(C.USAGE_MEDIA)
          .setContentType(C.AUDIO_CONTENT_TYPE_MUSIC)
          .build(),
        true,
      )
      addListener(this@PlaybackService)
    }
    setMediaNotificationProvider(
      DefaultMediaNotificationProvider.Builder(this)
        .build(),
    )
    sessionPlayer = QueueCommandForwardingPlayer(player, rustBridge)
    val sessionActivityIntent =
      PendingIntent.getActivity(
        this,
        0,
        Intent(this, MainActivity::class.java).apply {
          flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
        },
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
      )
    mediaSession = MediaSession.Builder(this, sessionPlayer)
      .setSessionActivity(sessionActivityIntent)
      .build()
    PlaybackServiceHost.publish(this)
  }

  override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaSession? {
    return mediaSession
  }

  override fun onDestroy() {
    pendingLoad?.callback?.invoke(Result.failure(IllegalStateException("Android playback service was destroyed.")))
    pendingLoad = null
    PlaybackServiceHost.clear(this)
    mediaSession?.release()
    player.removeListener(this)
    player.release()
    mediaSession = null
    super.onDestroy()
  }

  fun loadPreparedMedia(request: LoadPreparedMediaArgs, callback: (Result<Unit>) -> Unit) {
    pendingLoad?.callback?.invoke(Result.failure(IllegalStateException("Android playback load was superseded.")))
    pendingLoad = PendingLoad(request.mediaId, callback)
    loadedStreamBasePositionMs =
      (request.absoluteStartPositionMs - request.localStartPositionMs).coerceAtLeast(0)

    val metadataBuilder = MediaMetadata.Builder()
      .setTitle(request.title)
      .setArtist(request.artist)
      .setAlbumTitle(request.album)
    request.artworkUrl
      ?.takeIf { artworkUrl -> artworkUrl.isNotBlank() }
      ?.let { artworkUrl -> metadataBuilder.setArtworkUri(Uri.parse(artworkUrl)) }

    val mediaItem = MediaItem.Builder()
      .setMediaId(request.mediaId)
      .setUri(request.streamUrl)
      .setMediaMetadata(metadataBuilder.build())
      .build()

    val requestHeaders = linkedMapOf<String, String>()
    request.headers.forEach { header ->
      if (header.name.isNotBlank()) {
        requestHeaders[header.name] = header.value
      }
    }

    val dataSourceFactory = DefaultHttpDataSource.Factory()
      .setAllowCrossProtocolRedirects(true)
      .setDefaultRequestProperties(requestHeaders)
    val mediaSource = ProgressiveMediaSource.Factory(dataSourceFactory)
      .createMediaSource(mediaItem)

    player.stop()
    player.clearMediaItems()
    player.setMediaSource(mediaSource, request.localStartPositionMs)
    player.playWhenReady = request.autoplay
    player.prepare()
  }

  fun pause() {
    player.pause()
  }

  fun stopPlayback() {
    pendingLoad?.callback?.invoke(Result.failure(IllegalStateException("Android playback was interrupted.")))
    pendingLoad = null
    player.stop()
    player.clearMediaItems()
    loadedStreamBasePositionMs = 0
  }

  fun seekToAbsolutePosition(positionMs: Long) {
    val localPositionMs = (positionMs - loadedStreamBasePositionMs).coerceAtLeast(0)
    player.seekTo(localPositionMs)
  }

  fun currentAbsolutePositionMs(): Long {
    val currentPositionMs = player.currentPosition
    val safeCurrentPositionMs = if (currentPositionMs < 0) 0 else currentPositionMs
    return loadedStreamBasePositionMs + safeCurrentPositionMs
  }

  override fun onPlaybackStateChanged(playbackState: Int) {
    when (playbackState) {
      Player.STATE_BUFFERING -> {
        rustBridge.enqueuePlaybackEvent("buffering", currentAbsolutePositionMs(), null)
      }

      Player.STATE_READY -> {
        pendingLoad?.callback?.invoke(Result.success(Unit))
        pendingLoad = null
        rustBridge.enqueuePlaybackEvent("ready", currentAbsolutePositionMs(), null)
        if (!player.playWhenReady) {
          rustBridge.enqueuePlaybackEvent("paused", currentAbsolutePositionMs(), null)
        }
      }

      Player.STATE_ENDED -> {
        pendingLoad = null
        rustBridge.enqueuePlaybackEvent("ended", currentAbsolutePositionMs(), null)
      }
    }
  }

  override fun onIsPlayingChanged(isPlaying: Boolean) {
    if (isPlaying) {
      rustBridge.enqueuePlaybackEvent("playing", currentAbsolutePositionMs(), null)
      return
    }

    if (player.playbackState == Player.STATE_READY && !player.playWhenReady) {
      rustBridge.enqueuePlaybackEvent("paused", currentAbsolutePositionMs(), null)
    }
  }

  override fun onPlayerError(error: PlaybackException) {
    val pending = pendingLoad
    pendingLoad = null
    if (pending != null) {
      pending.callback(Result.failure(IllegalStateException(error.message ?: "Android playback failed.")))
      return
    }

    rustBridge.enqueuePlaybackEvent(
      "error",
      currentAbsolutePositionMs(),
      error.message ?: "Android playback failed.",
    )
  }
}
