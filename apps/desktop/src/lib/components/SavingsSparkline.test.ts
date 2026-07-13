import { render } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import SavingsSparkline from './SavingsSparkline.svelte';

describe('SavingsSparkline', () => {
  it('renders a savings line for analytics trend data', () => {
    const { container } = render(SavingsSparkline, {
      points: [
        { period: '2026-07-10', saved_bytes: 0, optimized_count: 0 },
        { period: '2026-07-11', saved_bytes: 2_500, optimized_count: 2 },
      ],
    });

    expect(container.querySelector('svg')?.getAttribute('aria-label')).toBe('Savings over the last seven days');
    expect(container.querySelectorAll('path')).toHaveLength(2);
  });

  it('renders a calm empty state when no savings exist yet', () => {
    const { container } = render(SavingsSparkline, {
      points: [{ period: '2026-07-10', saved_bytes: 0, optimized_count: 0 }],
    });

    expect(container.querySelectorAll('path')).toHaveLength(1);
  });
});
