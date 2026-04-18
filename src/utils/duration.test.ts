import { describe, expect, it } from 'vitest';

import { formatClockTime, formatCompactDuration } from './duration';

describe('duration formatters', () => {
  it('formats playback clock time with floor-based minute handling', () => {
    expect(formatClockTime(584)).toBe('09:44');
  });

  it('formats compact list durations without rounding minutes up', () => {
    expect(formatCompactDuration(584)).toBe('9:44');
  });

  it('keeps shorter durations readable in compact mode', () => {
    expect(formatCompactDuration(23)).toBe('0:23');
    expect(formatCompactDuration(503)).toBe('8:23');
  });
});
