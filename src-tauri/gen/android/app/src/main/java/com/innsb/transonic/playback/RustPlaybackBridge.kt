package com.innsb.transonic.playback

class RustPlaybackBridge {
  external fun enqueuePlaybackEvent(kind: String, positionMs: Long, error: String?)

  external fun dispatchControllerAction(action: String)

  companion object {
    init {
      System.loadLibrary("transonic_lib")
    }
  }
}
