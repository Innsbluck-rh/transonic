import { describe, expect, it } from 'vitest';
import { orUnknown, UNKNOWN_VALUE } from '~/utils/unknownValue';

describe('orUnknown', () => {
  it('uses the placeholder for null, undefined and an empty string', () => {
    expect(orUnknown(null)).toBe(UNKNOWN_VALUE);
    expect(orUnknown(undefined)).toBe(UNKNOWN_VALUE);
    expect(orUnknown('')).toBe(UNKNOWN_VALUE);
  });

  it('keeps a server-supplied placeholder such as Navidrome’s', () => {
    expect(orUnknown('[Unknown Artist]')).toBe('[Unknown Artist]');
  });
});
