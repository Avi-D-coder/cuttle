import { parseCardToken } from '@/util/cutthroat-cards';

const SCRAP_TOKEN_PATTERN = /^(J[01]|[A23456789TJQK][CDHS])$/;

export function normalizeScrapToken(entry) {
  if (typeof entry !== 'string') {return null;}
  const token = entry.trim();
  if (!SCRAP_TOKEN_PATTERN.test(token)) {return null;}
  return token;
}

export function mapScrapEntriesToCards(scrapEntries = []) {
  if (!Array.isArray(scrapEntries)) {return [];}
  return scrapEntries.map((entry, index) => {
    const token = normalizeScrapToken(entry);
    const parsed = token ? parseCardToken(token) : null;
    if (parsed?.kind === 'joker') {
      return {
        id: `scrap-${index}-${token}`,
        token,
        rank: 14,
        suit: parsed.id,
      };
    }
    if (parsed?.kind === 'standard') {
      return {
        id: `scrap-${index}-${token}`,
        token,
        rank: parsed.rank,
        suit: parsed.suit,
      };
    }
    return {
      id: `scrap-${index}-unknown`,
      token: null,
      rank: undefined,
      suit: undefined,
    };
  });
}

export function isPlayableScrapToken(token) {
  return normalizeScrapToken(token) !== null;
}
