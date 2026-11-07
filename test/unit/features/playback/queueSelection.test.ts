import { describe, expect, it } from 'vitest';
import {
  addQueueIndex,
  emptyQueueSelection,
  isQueueMoveNoop,
  remapQueueSelection,
  selectAllQueue,
  selectQueueRange,
  selectQueueSpan,
  toggleQueueIndex,
} from '~/features/playback/queueSelection';

describe('queueSelection', () => {
  it('toggles rows and moves the anchor to the toggled row, on or off', () => {
    let selection = toggleQueueIndex(emptyQueueSelection, 3);
    selection = toggleQueueIndex(selection, 1);
    expect(selection).toEqual({ indices: [1, 3], anchor: 1 });

    selection = toggleQueueIndex(selection, 3);
    expect(selection).toEqual({ indices: [1], anchor: 3 });
  });

  it('ends the selection, anchor included, when the last row is turned off', () => {
    expect(toggleQueueIndex({ indices: [2], anchor: 0 }, 2)).toEqual(emptyQueueSelection);
  });

  it('starts a Shift range at the clicked row when nothing is selected', () => {
    expect(selectQueueRange(emptyQueueSelection, 4)).toEqual({ indices: [4], anchor: 4 });
  });

  it('replaces the selection with anchor..row on Shift+click and keeps the anchor', () => {
    const selection = selectQueueRange({ indices: [0, 2, 7], anchor: 2 }, 5);
    expect(selection).toEqual({ indices: [2, 3, 4, 5], anchor: 2 });
    expect(selectQueueRange(selection, 0)).toEqual({ indices: [0, 1, 2], anchor: 2 });
  });

  it('adds a row without moving the anchor', () => {
    expect(addQueueIndex({ indices: [1], anchor: 1 }, 4)).toEqual({ indices: [1, 4], anchor: 1 });
  });

  it('replaces the selection with the rows a rectangle covers', () => {
    expect(selectQueueSpan({ from: 5, to: 2 }, 5)).toEqual({ indices: [2, 3, 4, 5], anchor: 5 });
    expect(selectQueueSpan(null, 5)).toEqual(emptyQueueSelection);
  });

  it('adds the rows a rectangle covers to a base selection', () => {
    expect(selectQueueSpan({ from: 4, to: 5 }, 4, [0, 5, 8])).toEqual({ indices: [0, 4, 5, 8], anchor: 4 });
    expect(selectQueueSpan(null, 4, [0, 8])).toEqual({ indices: [0, 8], anchor: 4 });
  });

  it('selects every row with the first as anchor', () => {
    expect(selectAllQueue(3)).toEqual({ indices: [0, 1, 2], anchor: 0 });
    expect(selectAllQueue(0)).toEqual(emptyQueueSelection);
  });

  it('moves the selection and the anchor along a queue index map', () => {
    const indexMap = [0, 3, 1, 4, 2];
    expect(remapQueueSelection({ indices: [2, 4], anchor: 4 }, indexMap)).toEqual({ indices: [1, 2], anchor: 2 });
  });

  it('drops removed rows, and the anchor when it was removed', () => {
    expect(remapQueueSelection({ indices: [0, 1, 3], anchor: 1 }, [0, -1, 1, 2])).toEqual({ indices: [0, 2], anchor: null });
    expect(remapQueueSelection({ indices: [1], anchor: 1 }, [0, -1])).toEqual(emptyQueueSelection);
  });

  it('detects moves that leave the queue unchanged', () => {
    expect(isQueueMoveNoop([2, 3], 2)).toBe(true);
    expect(isQueueMoveNoop([2, 3], 4)).toBe(true);
    expect(isQueueMoveNoop([2, 3], 5)).toBe(false);
    expect(isQueueMoveNoop([2, 4], 3)).toBe(false);
  });
});
