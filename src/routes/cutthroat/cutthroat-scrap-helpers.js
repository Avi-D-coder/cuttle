import { parseCardToken } from '@/util/cutthroat-cards';

export function mapScrapEntriesToCards(scrapEntries = []) {
  const normalizedEntries = normalizeScrapEntries(scrapEntries);
  return normalizedEntries.map((entry, index) => {
    const token = normalizeScrapToken(entry);

    if (token) {
      const parsedCard = parseCardToken(token);
      if (parsedCard?.kind === 'joker') {
        return {
          id: `scrap-${index}-${token}`,
          token,
          rank: 14,
          suit: parsedCard.id,
        };
      }

      if (parsedCard?.kind === 'standard') {
        return {
          id: `scrap-${index}-${token}`,
          token,
          rank: parsedCard.rank,
          suit: parsedCard.suit,
        };
      }
    }

    const fallbackCard = extractRankSuitCard(entry);
    return {
      id: `scrap-${index}-${token ?? 'unknown'}`,
      token: token ?? null,
      rank: fallbackCard?.rank,
      suit: fallbackCard?.suit,
    };
  });
}

function normalizeScrapEntries(scrapEntries) {
  if (Array.isArray(scrapEntries)) {
    return scrapEntries;
  }

  if (!scrapEntries || typeof scrapEntries !== 'object') {
    return [];
  }

  return Object.keys(scrapEntries)
    .sort((a, b) => {
      const aNumeric = Number(a);
      const bNumeric = Number(b);
      const aIsNumeric = Number.isInteger(aNumeric) && String(aNumeric) === a;
      const bIsNumeric = Number.isInteger(bNumeric) && String(bNumeric) === b;

      if (aIsNumeric && bIsNumeric) {
        return aNumeric - bNumeric;
      }
      if (aIsNumeric) {return -1;}
      if (bIsNumeric) {return 1;}
      return a.localeCompare(b);
    })
    .map((key) => scrapEntries[key]);
}

export function normalizeScrapToken(entry) {
  if (!entry) {return null;}

  if (typeof entry === 'string') {
    return normalizeTokenString(entry);
  }

  if (typeof entry === 'object') {
    if (entry.Standard || entry.standard) {
      const standard = entry.Standard ?? entry.standard;
      const rank = rankToTokenChar(standard?.rank);
      const suit = suitToTokenChar(standard?.suit);
      if (rank && suit) {return `${rank}${suit}`;}
    }

    if (entry.Joker !== undefined || entry.joker !== undefined) {
      const jokerValue = entry.Joker ?? entry.joker;
      const jokerId = Number(jokerValue);
      if (jokerId === 0 || jokerId === 1) {return `J${jokerId}`;}
    }

    if (typeof entry.data === 'string') {
      return normalizeTokenString(entry.data);
    }

    if (typeof entry.token === 'string') {
      return normalizeTokenString(entry.token);
    }

    if (typeof entry.card === 'string') {
      return normalizeTokenString(entry.card);
    }

    if (entry.rank !== undefined && entry.suit !== undefined) {
      const rank = rankToTokenChar(entry.rank);
      const suit = suitToTokenChar(entry.suit);
      if (rank && suit) {return `${rank}${suit}`;}
    }

    const nestedToken = deepFindToken(entry);
    if (nestedToken) {return nestedToken;}

    // Last fallback for shape drift: parse first card token from serialized object.
    try {
      const serialized = JSON.stringify(entry);
      const matched = extractTokenFromText(serialized);
      if (matched) {return matched;}
    } catch (_) {
      // ignore non-serializable payloads
    }
  }

  return null;
}

export function normalizeTokenString(rawToken) {
  const token = String(rawToken)
    .trim()
    .toUpperCase();

  if (/^J[01]$/.test(token)) {return token;}
  if (/^(10|[2-9AJQKT])[CDHS]$/.test(token)) {
    if (token.startsWith('10')) {
      return `T${token[2]}`;
    }
    return token;
  }
  return null;
}

export function rankToTokenChar(rankValue) {
  const normalized = String(rankValue)
    .trim()
    .toUpperCase();
  const mapped = {
    A: 'A',
    ACE: 'A',
    1: 'A',
    2: '2',
    TWO: '2',
    3: '3',
    THREE: '3',
    4: '4',
    FOUR: '4',
    5: '5',
    FIVE: '5',
    6: '6',
    SIX: '6',
    7: '7',
    SEVEN: '7',
    8: '8',
    EIGHT: '8',
    9: '9',
    NINE: '9',
    10: 'T',
    T: 'T',
    TEN: 'T',
    J: 'J',
    JACK: 'J',
    11: 'J',
    Q: 'Q',
    QUEEN: 'Q',
    12: 'Q',
    K: 'K',
    KING: 'K',
    13: 'K',
  }[normalized];

  return mapped ?? null;
}

export function suitToTokenChar(suitValue) {
  const normalized = String(suitValue)
    .trim()
    .toUpperCase();
  return {
    0: 'C',
    C: 'C',
    CLUBS: 'C',
    1: 'D',
    D: 'D',
    DIAMONDS: 'D',
    2: 'H',
    H: 'H',
    HEARTS: 'H',
    3: 'S',
    S: 'S',
    SPADES: 'S',
  }[normalized] ?? null;
}

export function deepFindToken(value, seen = new Set()) {
  if (!value) {return null;}
  if (typeof value === 'string') {return normalizeTokenString(value);}
  if (typeof value !== 'object') {return null;}
  if (seen.has(value)) {return null;}
  seen.add(value);

  if (Array.isArray(value)) {
    for (const item of value) {
      const token = deepFindToken(item, seen);
      if (token) {return token;}
    }
    return null;
  }

  const rank = rankToTokenChar(value.rank);
  const suit = suitToTokenChar(value.suit);
  if (rank && suit) {return `${rank}${suit}`;}

  for (const nested of Object.values(value)) {
    const token = deepFindToken(nested, seen);
    if (token) {return token;}
  }

  return null;
}

export function extractRankSuitCard(entry) {
  const token = deepFindToken(entry);
  if (!token) {return null;}
  const parsed = parseCardToken(token);
  if (parsed?.kind !== 'standard') {return null;}
  return parsed;
}

export function isPlayableScrapToken(token) {
  if (typeof token !== 'string') {return false;}
  return !!normalizeTokenString(token);
}

function extractTokenFromText(value) {
  if (!value || typeof value !== 'string') {return null;}
  const normalized = value
    .toUpperCase()
    .replace(/\\u0000/g, '');
  const match = normalized.match(/(?<![A-Z0-9])(J[01]|10[CDHS]|[2-9AJQKT][CDHS])(?![A-Z0-9])/);
  if (!match) {return null;}
  return normalizeTokenString(match[0]);
}
