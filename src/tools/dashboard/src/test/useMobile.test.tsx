import React from 'react';
import { renderHook, act } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { useIsMobile } from '../components/ui/use-mobile';

function setWindowWidth(width: number) {
  // @ts-ignore
  window.innerWidth = width;
  window.dispatchEvent(new Event('resize'));
}

describe('useIsMobile', () => {
  it('returns true when width is below breakpoint', () => {
    setWindowWidth(500);
    const { result } = renderHook(() => useIsMobile());
    expect(result.current).toBe(true);
  });

  it('returns false when width is above breakpoint', () => {
    setWindowWidth(1024);
    const { result } = renderHook(() => useIsMobile());
    expect(result.current).toBe(false);
  });
});
