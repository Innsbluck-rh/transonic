package com.innsb.transonic.playback

import android.app.PendingIntent
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.core.content.FileProvider
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.ForwardingPlayer
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.TrackSelectionParameters.AudioOffloadPreferences
import androidx.media3.common.util.UnstableApi
import androidx.media3.datasource.DefaultHttpDataSource
import androidx.media3.exoplayer.DefaultRenderersFactory
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.source.ProgressiveMediaSource
import androidx.media3.session.DefaultMediaNotificationProvider
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSession.ConnectionResult
import androidx.media3.session.MediaSessionService
import androidx.media3.session.SessionCommand
import androidx.media3.session.SessionResult
import com.innsb.transonic.BuildConfig
import com.innsb.transonic.MainActivity
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import com.google.common.util.concurrent.SettableFuture
import java.io.File

private data class PendingLoad(
  val mediaId: String,
  val callback: (Result<Unit>) -> Unit,
)

private data class PlaybackSlotState(
  val mediaId: String,
  val basePositionMs: Long,
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
  companion object {
    const val EXTRA_FROM_NOTIFICATION = "from_notification"
  }

  private val rustBridge = RustPlaybackBridge()
  private lateinit var player: ExoPlayer
  private lateinit var sessionPlayer: QueueCommandForwardingPlayer
  private var mediaSession: MediaSession? = null
  private var pendingLoad: PendingLoad? = null
  private var currentMediaState: PlaybackSlotState? = null
  private var preparedMediaState: PlaybackSlotState? = null
  private var transitioningMediaState: PlaybackSlotState? = null
  private var manualTransitionPendingMediaId: String? = null
  private var suppressTransitionMediaId: String? = null

  override fun onCreate() {
    super.onCreate()

    player = ExoPlayer.Builder(this)
      .setRenderersFactory(
        DefaultRenderersFactory(this)
          .setEnableDecoderFallback(true)
      )
      .setHandleAudioBecomingNoisy(true)
      .build()
      .apply {
        trackSelectionParameters =
          trackSelectionParameters
            .buildUpon()
            .setAudioOffloadPreferences(
              AudioOffloadPreferences.Builder()
                .setAudioOffloadMode(AudioOffloadPreferences.AUDIO_OFFLOAD_MODE_DISABLED)
                .build(),
            )
            .build()
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
          putExtra(EXTRA_FROM_NOTIFICATION, true)
        },
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
      )
    mediaSession = MediaSession.Builder(this, sessionPlayer)
      .setCallback(PlaybackSessionCallback(this))
      .setSessionActivity(sessionActivityIntent)
      .build()
  }

  override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaSession? {
    return mediaSession
  }

  override fun onDestroy() {
    pendingLoad?.callback?.invoke(Result.failure(IllegalStateException("Android playback service was destroyed.")))
    pendingLoad = null
    currentMediaState = null
    preparedMediaState = null
    transitioningMediaState = null
    manualTransitionPendingMediaId = null
    suppressTransitionMediaId = null
    mediaSession?.release()
    player.removeListener(this)
    player.release()
    mediaSession = null
    super.onDestroy()
  }

  fun loadPreparedMedia(request: LoadPreparedMediaArgs, callback: (Result<Unit>) -> Unit) {
    pendingLoad?.callback?.invoke(Result.failure(IllegalStateException("Android playback load was superseded.")))
    pendingLoad = PendingLoad(request.mediaId, callback)
    currentMediaState = request.toPlaybackSlotState()
    preparedMediaState = null
    transitioningMediaState = null
    manualTransitionPendingMediaId = null
    suppressTransitionMediaId = null

    val mediaSource = buildMediaSource(request)
    player.stop()
    player.clearMediaItems()
    player.setMediaSource(mediaSource, request.localStartPositionMs)
    player.playWhenReady = request.autoplay
    player.prepare()
  }

  fun prepareNextMedia(request: LoadPreparedMediaArgs): Result<Unit> {
    if (currentMediaState == null || player.mediaItemCount == 0) {
      return Result.failure(IllegalStateException("No current Android playback media is loaded."))
    }

    val priorPreparedMediaId = preparedMediaState?.mediaId
    preparedMediaState = request.toPlaybackSlotState()
    if (priorPreparedMediaId != null) {
      val priorPreparedIndex = findMediaItemIndex(priorPreparedMediaId)
      if (priorPreparedIndex != C.INDEX_UNSET) {
        player.removeMediaItem(priorPreparedIndex)
      }
    }

    val insertIndex = (preparedAnchorIndex() + 1).coerceAtMost(player.mediaItemCount)
    player.addMediaSource(insertIndex, buildMediaSource(request))
    return Result.success(Unit)
  }

  fun activatePreparedMedia(request: ActivatePreparedMediaArgs): Result<Unit> {
    val prepared = preparedMediaState
      ?: return Result.failure(IllegalStateException("No prepared Android playback media is available."))
    if (player.mediaItemCount < 2) {
      return Result.failure(IllegalStateException("Prepared Android playback media is missing from the playlist."))
    }

    transitioningMediaState = prepared
    preparedMediaState = null
    manualTransitionPendingMediaId = prepared.mediaId
    player.playWhenReady = request.autoplay
    player.seekToNextMediaItem()
    return Result.success(Unit)
  }

  fun clearPreparedMedia(): Result<Unit> {
    val preparedMediaId = preparedMediaState?.mediaId
    preparedMediaState = null
    manualTransitionPendingMediaId = null
    if (preparedMediaId != null) {
      val preparedIndex = findMediaItemIndex(preparedMediaId)
      if (preparedIndex != C.INDEX_UNSET) {
        player.removeMediaItem(preparedIndex)
      }
    }
    return Result.success(Unit)
  }

  fun updateMediaArtwork(request: UpdateMediaArtworkArgs): Result<Unit> {
    if (request.mediaId.isBlank() || request.artworkPath.isBlank()) {
      return Result.success(Unit)
    }

    val currentIndex = player.currentMediaItemIndex
    if (currentIndex == C.INDEX_UNSET || currentIndex >= player.mediaItemCount) {
      return Result.success(Unit)
    }

    val mediaItem = player.getMediaItemAt(currentIndex)
    if (!mediaItemMatches(mediaItem.mediaId, request.mediaId)) {
      return Result.success(Unit)
    }

    val artworkUri = artworkContentUri(request.artworkPath) ?: return Result.success(Unit)
    val mediaMetadata = mediaItem.mediaMetadata
      .buildUpon()
      .setArtworkUri(artworkUri)
      .build()
    player.replaceMediaItem(
      currentIndex,
      mediaItem
        .buildUpon()
        .setMediaMetadata(mediaMetadata)
        .build(),
    )
    return Result.success(Unit)
  }

  fun currentAbsolutePositionMs(): Long {
    val currentPositionMs = player.currentPosition
    val safeCurrentPositionMs = if (currentPositionMs < 0) 0 else currentPositionMs
    return (currentMediaState?.basePositionMs ?: 0) + safeCurrentPositionMs
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
        preparedMediaState = null
        transitioningMediaState = null
        manualTransitionPendingMediaId = null
        rustBridge.enqueuePlaybackEvent("ended", currentAbsolutePositionMs(), null)
      }
    }
  }

  override fun onMediaItemTransition(mediaItem: MediaItem?, reason: Int) {
    val transitionedMediaId = mediaItem?.mediaId ?: return
    if (suppressTransitionMediaId == transitionedMediaId) {
      suppressTransitionMediaId = null
      return
    }

    val transitionState =
      when {
        transitioningMediaState?.mediaId == transitionedMediaId -> transitioningMediaState
        preparedMediaState?.mediaId == transitionedMediaId -> preparedMediaState
        else -> null
      }
        ?: return

    if (transitioningMediaState?.mediaId == transitionedMediaId) {
      transitioningMediaState = null
    } else {
      preparedMediaState = null
    }

    currentMediaState = transitionState
    val transitionIndex = player.currentMediaItemIndex
    if (transitionIndex == C.INDEX_UNSET) {
      return
    }

    val isManual =
      manualTransitionPendingMediaId == transitionedMediaId ||
        reason == Player.MEDIA_ITEM_TRANSITION_REASON_SEEK
    manualTransitionPendingMediaId = null
    if (transitionIndex > 0 && player.mediaItemCount > transitionIndex) {
      suppressTransitionMediaId = transitionedMediaId
      player.removeMediaItem(0)
    }
    if (!isManual) {
      rustBridge.enqueuePlaybackEvent("gapless_transition", currentAbsolutePositionMs(), null)
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
      pending.callback(
        Result.failure(
          IllegalStateException(
            error.message ?: "Android playback failed."
          )
        )
      )
      return
    }

    preparedMediaState = null
    transitioningMediaState = null
    manualTransitionPendingMediaId = null
    rustBridge.enqueuePlaybackEvent(
      "error",
      currentAbsolutePositionMs(),
      error.message ?: "Android playback failed.",
    )
  }

  private fun buildMediaSource(request: LoadPreparedMediaArgs): ProgressiveMediaSource {
    val metadataBuilder = MediaMetadata.Builder()
      .setTitle(request.title)
      .setArtist(request.artist)
      .setAlbumTitle(request.album)
    request.artworkPath
      ?.let { artworkPath -> artworkContentUri(artworkPath) }
      ?.let { artworkUri -> metadataBuilder.setArtworkUri(artworkUri) }

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
    return ProgressiveMediaSource.Factory(dataSourceFactory)
      .createMediaSource(mediaItem)
  }

  private fun LoadPreparedMediaArgs.toPlaybackSlotState(): PlaybackSlotState {
    return PlaybackSlotState(
      mediaId = mediaId,
      basePositionMs = (absoluteStartPositionMs - localStartPositionMs).coerceAtLeast(0),
    )
  }

  private fun preparedAnchorIndex(): Int {
    val transitioningMediaId = transitioningMediaState?.mediaId
    if (transitioningMediaId != null) {
      val transitioningIndex = findMediaItemIndex(transitioningMediaId)
      if (transitioningIndex != C.INDEX_UNSET) {
        return transitioningIndex
      }
    }

    val currentMediaId = currentMediaState?.mediaId
    if (currentMediaId != null) {
      val currentIndex = findMediaItemIndex(currentMediaId)
      if (currentIndex != C.INDEX_UNSET) {
        return currentIndex
      }
    }

    val fallbackIndex = player.currentMediaItemIndex
    return if (fallbackIndex != C.INDEX_UNSET) fallbackIndex else 0
  }

  private fun findMediaItemIndex(mediaId: String): Int {
    for (index in 0 until player.mediaItemCount) {
      if (player.getMediaItemAt(index).mediaId == mediaId) {
        return index
      }
    }
    return C.INDEX_UNSET
  }

  private fun mediaItemMatches(actualMediaId: String, requestedMediaId: String): Boolean {
    return actualMediaId == requestedMediaId ||
      actualMediaId.substringBefore("#") == requestedMediaId
  }

  private fun artworkContentUri(artworkPath: String): Uri? {
    if (artworkPath.isBlank()) {
      return null
    }

    val artworkFile = File(artworkPath)
    if (!artworkFile.exists()) {
      return null
    }

    return FileProvider.getUriForFile(
      this,
      "${BuildConfig.APPLICATION_ID}.fileprovider",
      artworkFile,
    )
  }
}

@UnstableApi
private class PlaybackSessionCallback(
  private val service: PlaybackService,
) : MediaSession.Callback {
  override fun onConnect(
    session: MediaSession,
    controller: MediaSession.ControllerInfo,
  ): ConnectionResult {
    val sessionCommands = ConnectionResult.DEFAULT_SESSION_COMMANDS.buildUpon()
    if (controller.packageName == service.packageName) {
      sessionCommands.add(SessionCommand(COMMAND_LOAD_PREPARED_MEDIA, Bundle.EMPTY))
      sessionCommands.add(SessionCommand(COMMAND_PREPARE_NEXT_MEDIA, Bundle.EMPTY))
      sessionCommands.add(SessionCommand(COMMAND_ACTIVATE_PREPARED_MEDIA, Bundle.EMPTY))
      sessionCommands.add(SessionCommand(COMMAND_CLEAR_PREPARED_MEDIA, Bundle.EMPTY))
      sessionCommands.add(SessionCommand(COMMAND_UPDATE_MEDIA_ARTWORK, Bundle.EMPTY))
    }

    return ConnectionResult.AcceptedResultBuilder(session)
      .setAvailableSessionCommands(sessionCommands.build())
      .build()
  }

  override fun onCustomCommand(
    session: MediaSession,
    controller: MediaSession.ControllerInfo,
    customCommand: SessionCommand,
    args: Bundle,
  ): ListenableFuture<SessionResult> {
    return when (customCommand.customAction) {
      COMMAND_LOAD_PREPARED_MEDIA -> handleLoadPreparedMedia(args)
      COMMAND_PREPARE_NEXT_MEDIA -> {
        val request =
          args.toLoadPreparedMediaArgs()
            ?: return Futures.immediateFailedFuture(
              IllegalArgumentException("Missing Media3 custom command payload for next media preparation."),
            )
        handleImmediateResult(service.prepareNextMedia(request))
      }

      COMMAND_ACTIVATE_PREPARED_MEDIA -> handleImmediateResult(service.activatePreparedMedia(args.toActivatePreparedMediaArgs()))
      COMMAND_CLEAR_PREPARED_MEDIA -> handleImmediateResult(service.clearPreparedMedia())
      COMMAND_UPDATE_MEDIA_ARTWORK -> {
        val request =
          args.toUpdateMediaArtworkArgs()
            ?: return Futures.immediateFailedFuture(
              IllegalArgumentException("Missing Media3 custom command payload for artwork update."),
            )
        handleImmediateResult(service.updateMediaArtwork(request))
      }

      else -> Futures.immediateFuture(SessionResult(SessionResult.RESULT_ERROR_NOT_SUPPORTED))
    }
  }

  private fun handleLoadPreparedMedia(args: Bundle): ListenableFuture<SessionResult> {
    val request =
      args.toLoadPreparedMediaArgs()
        ?: return Futures.immediateFailedFuture(
          IllegalArgumentException("Missing Media3 custom command payload for playback load."),
        )

    val future = SettableFuture.create<SessionResult>()
    service.loadPreparedMedia(request) { result ->
      result.onSuccess {
        future.set(SessionResult(SessionResult.RESULT_SUCCESS))
      }.onFailure { error ->
        future.setException(error)
      }
    }
    return future
  }

  private fun handleImmediateResult(result: Result<Unit>): ListenableFuture<SessionResult> {
    return result.fold(
      onSuccess = { Futures.immediateFuture(SessionResult(SessionResult.RESULT_SUCCESS)) },
      onFailure = { error -> Futures.immediateFailedFuture(error) },
    )
  }
}
