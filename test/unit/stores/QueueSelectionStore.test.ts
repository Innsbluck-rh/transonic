import { beforeEach, describe, expect, it } from 'vitest';
import { setPlaybackStore } from '~/stores/PlaybackStore';
import { clearQueueSelection, queueSelection, selectQueueRangeTo, toggleQueueSelection } from '~/stores/QueueSelectionStore';
import { resetConnectStore } from '~test/fixtures/connect';
import { playbackStatus, song } from '~test/fixtures/playback';

const queue = ['a', 'b', 'c', 'd'].map((id) => song(id));

describe('QueueSelectionStore', () => {
  beforeEach(() => {
    resetConnectStore();
    setPlaybackStore('status', playbackStatus({ queue, currentIndex: 0, queueRevision: 10 }));
    clearQueueSelection();
  });

  it('follows the index map of the next queue revision', () => {
    toggleQueueSelection(1);
    toggleQueueSelection(3);

    setPlaybackStore(
      'status',
      playbackStatus({ queue: [queue[0], queue[2], queue[3]], currentIndex: 0, queueRevision: 11, queueIndexMap: [0, -1, 1, 2] })
    );

    expect(queueSelection()).toEqual({ indices: [2], anchor: 2 });
  });

  it('ends the selection when a revision is skipped', () => {
    selectQueueRangeTo(2);

    setPlaybackStore('status', playbackStatus({ queue, currentIndex: 0, queueRevision: 12, queueIndexMap: [0, 1, 2, 3] }));

    expect(queueSelection().indices).toEqual([]);
  });

  it('ends the selection when the queue changes without a new revision', () => {
    selectQueueRangeTo(2);

    setPlaybackStore('status', playbackStatus({ queue: [queue[1], queue[0], queue[2], queue[3]], currentIndex: 0, queueRevision: 10 }));

    expect(queueSelection().indices).toEqual([]);
  });

  it('keeps the selection across status updates that leave the queue alone', () => {
    selectQueueRangeTo(2);

    setPlaybackStore('status', playbackStatus({ queue, currentIndex: 1, queueRevision: 10, currentPositionMs: 5000 }));

    expect(queueSelection()).toEqual({ indices: [2], anchor: 2 });
  });
});
