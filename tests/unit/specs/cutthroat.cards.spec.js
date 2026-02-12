import { describe, expect, it } from 'vitest';
import {
  publicCardToken,
  publicCardToDisplay,
} from '@/util/cutthroat-cards';

describe('cutthroat card wire parsing', () => {
  it('parses known card from rust external-tagged shape', () => {
    expect(publicCardToken({ Known: 'QD' })).toBe('QD');
    expect(publicCardToDisplay({ Known: 'QD' })).toEqual({
      kind: 'standard',
      rank: 12,
      suit: 1,
    });
  });

  it('treats rust hidden variant as hidden card', () => {
    expect(publicCardToken('Hidden')).toBe(null);
    expect(publicCardToDisplay('Hidden')).toEqual({ kind: 'hidden' });
  });

  it('rejects legacy tagged object variants', () => {
    expect(publicCardToken({ type: 'Known', data: '9C' })).toBe(null);
    expect(publicCardToDisplay({ type: 'Known', data: '9C' })).toEqual({ kind: 'hidden' });
    expect(publicCardToDisplay({ type: 'Hidden' })).toEqual({ kind: 'hidden' });
  });
});
