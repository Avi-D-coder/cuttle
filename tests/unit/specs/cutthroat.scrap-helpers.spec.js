import { describe, expect, it } from 'vitest';
import {
  isPlayableScrapToken,
  mapScrapEntriesToCards,
  normalizeScrapToken,
} from '@/routes/cutthroat/cutthroat-scrap-helpers';

describe('cutthroat scrap helpers', () => {
  it('normalizes known token string formats', () => {
    expect(normalizeScrapToken('4d')).toBe('4D');
    expect(normalizeScrapToken('10c')).toBe('TC');
    expect(normalizeScrapToken('j1')).toBe('J1');
  });

  it('normalizes rust enum card objects', () => {
    expect(normalizeScrapToken({ Standard: { rank: 'TEN', suit: 'Hearts' } })).toBe('TH');
    expect(normalizeScrapToken({ standard: { rank: 4, suit: 1 } })).toBe('4D');
    expect(normalizeScrapToken({ Joker: 0 })).toBe('J0');
  });

  it('finds nested token payloads', () => {
    const nested = {
      card: {
        data: {
          source: {
            Standard: {
              rank: 'ACE',
              suit: 'spades',
            },
          },
        },
      },
    };

    expect(normalizeScrapToken(nested)).toBe('AS');
  });

  it('maps mixed scrap entries to displayable cards without dropping entries', () => {
    const cards = mapScrapEntriesToCards([
      '4D',
      '10C',
      { Standard: { rank: 'QUEEN', suit: 'clubs' } },
      { data: 'J1' },
      { rank: 'ACE', suit: 'SPADES' },
      { weird: { unknown: true } },
    ]);

    expect(cards).toHaveLength(6);
    expect(cards[0]).toMatchObject({ token: '4D', rank: 4, suit: 1 });
    expect(cards[1]).toMatchObject({ token: 'TC', rank: 10, suit: 0 });
    expect(cards[2]).toMatchObject({ token: 'QC', rank: 12, suit: 0 });
    expect(cards[3]).toMatchObject({ token: 'J1', rank: 14, suit: 1 });
    expect(cards[4]).toMatchObject({ token: 'AS', rank: 1, suit: 3 });
    expect(cards[5]).toMatchObject({ token: null, rank: undefined, suit: undefined });
  });

  it('maps object-shaped scrap payloads from cutthroat state responses', () => {
    const objectShapedScrap = {
      0: { Standard: { rank: 'THREE', suit: 'CLUBS' } },
      1: { Standard: { rank: 'TEN', suit: 'DIAMONDS' } },
      2: { Joker: 1 },
    };

    const cards = mapScrapEntriesToCards(objectShapedScrap);

    expect(cards).toHaveLength(3);
    expect(cards[0]).toMatchObject({ token: '3C', rank: 3, suit: 0 });
    expect(cards[1]).toMatchObject({ token: 'TD', rank: 10, suit: 1 });
    expect(cards[2]).toMatchObject({ token: 'J1', rank: 14, suit: 1 });
  });

  it('extracts tokens when card values are embedded in object keys or text payloads', () => {
    const cards = mapScrapEntriesToCards([
      { '3C': { weird: true } },
      { payload: 'card_from_scrap=10D' },
    ]);

    expect(cards).toHaveLength(2);
    expect(cards[0]).toMatchObject({ token: '3C', rank: 3, suit: 0 });
    expect(cards[1]).toMatchObject({ token: 'TD', rank: 10, suit: 1 });
  });

  it('validates playable scrap tokens for resolve-three picks', () => {
    expect(isPlayableScrapToken('4D')).toBe(true);
    expect(isPlayableScrapToken('10H')).toBe(true);
    expect(isPlayableScrapToken('J0')).toBe(true);
    expect(isPlayableScrapToken('')).toBe(false);
    expect(isPlayableScrapToken(null)).toBe(false);
    expect(isPlayableScrapToken('bad')).toBe(false);
  });
});
