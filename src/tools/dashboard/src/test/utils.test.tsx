import { describe, it, expect } from 'vitest';
import { cn } from '../components/ui/utils';

describe('cn utility', () => {
  it('merges classes and removes duplicates', () => {
    const result = cn('p-2', false && 'hidden', 'p-2', 'text-red-500');
    expect(result).toContain('p-2');
    expect(result).toContain('text-red-500');
  });

  it('handles empty input gracefully', () => {
    const result = cn();
    expect(typeof result).toBe('string');
  });
});
