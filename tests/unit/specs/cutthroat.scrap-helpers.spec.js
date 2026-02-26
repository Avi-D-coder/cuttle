import { describe, expect, it } from 'vitest';
import {
  isPlayableScrapToken,
  mapScrapEntriesToCards,
  normalizeScrapToken,
} from '@/routes/cutthroat/cutthroat-scrap-helpers';

describe('cutthroat scrap helpers', () => {
  it('accepts rust scrap token strings only', () => {
    expect(normalizeScrapToken('4D')).toBe('4D');
    expect(normalizeScrapToken('TC')).toBe('TC');
    expect(normalizeScrapToken('J1')).toBe('J1');
  });

  it('rejects non-rust or coercion-requiring token shapes', () => {
    expect(normalizeScrapToken('4d')).toBe(null);
    expect(normalizeScrapToken('10C')).toBe(null);
    expect(normalizeScrapToken({ Standard: { rank: 'TEN', suit: 'H' } })).toBe(null);
    expect(normalizeScrapToken({ data: 'TC' })).toBe(null);
  });

  it('maps rust scrap token arrays to display cards', () => {
    const cards = mapScrapEntriesToCards([ '4D', 'TC', 'J0' ]);

    expect(cards).toHaveLength(3);
    expect(cards[0]).toMatchObject({ token: '4D', rank: 4, suit: 1 });
    expect(cards[1]).toMatchObject({ token: 'TC', rank: 10, suit: 0 });
    expect(cards[2]).toMatchObject({ token: 'J0', rank: 14, suit: 0 });
  });

  it('does not coerce invalid entries', () => {
    const cards = mapScrapEntriesToCards([ '4D', '10H', { token: 'TC' } ]);

    expect(cards).toHaveLength(3);
    expect(cards[0]).toMatchObject({ token: '4D', rank: 4, suit: 1 });
    expect(cards[1]).toMatchObject({ token: null, rank: undefined, suit: undefined });
    expect(cards[2]).toMatchObject({ token: null, rank: undefined, suit: undefined });
  });

  it('returns empty when scrap payload is not an array', () => {
    expect(mapScrapEntriesToCards({ 0: '4D' })).toEqual([]);
  });

  it('validates playable scrap tokens strictly', () => {
    expect(isPlayableScrapToken('4D')).toBe(true);
    expect(isPlayableScrapToken('TC')).toBe(true);
    expect(isPlayableScrapToken('J0')).toBe(true);
    expect(isPlayableScrapToken('10H')).toBe(false);
    expect(isPlayableScrapToken('4d')).toBe(false);
    expect(isPlayableScrapToken('bad')).toBe(false);
  });
});
