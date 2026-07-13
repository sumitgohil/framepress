import { describe, expect, it } from 'vitest';

import { format_bytes } from './format';

describe('format_bytes', () => {
  it('uses the correct unit at each 1024-byte boundary', () => {
    expect(format_bytes(0)).toBe('0.00 B');
    expect(format_bytes(1024)).toBe('1.00 KB');
    expect(format_bytes(1024 * 1024)).toBe('1.00 MB');
    expect(format_bytes(1024 * 1024 * 1024)).toBe('1.00 GB');
  });

  it('formats the uploaded leaderboard image size as megabytes', () => {
    expect(format_bytes(1_369_492)).toBe('1.31 MB');
  });
});
