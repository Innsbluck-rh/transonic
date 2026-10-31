import { describe, expect, it } from 'vitest';

describe('test placement', () => {
  it('keeps test files out of src (vitest only collects test/)', () => {
    const misplaced = Object.keys(import.meta.glob('/src/**/*.test.{ts,tsx}'));
    expect(misplaced).toEqual([]);
  });
});
