const RANKS = {
  A: 1,
  '2': 2,
  '3': 3,
  '4': 4,
  '5': 5,
  '6': 6,
  '7': 7,
  '8': 8,
  '9': 9,
  T: 10,
  J: 11,
  Q: 12,
  K: 13,
};

const SUITS = {
  C: 0,
  D: 1,
  H: 2,
  S: 3,
};

export function parseCardToken(token) {
  if (!token) {return null;}
  if (token === 'J0' || token === 'J1') {
    const [ , jokerId ] = token;
    return { kind: 'joker', id: Number(jokerId) };
  }
  if (token.length !== 2) {return { kind: 'hidden' };}
  const [ rankChar, suitChar ] = token;
  const rank = RANKS[rankChar];
  const suit = SUITS[suitChar];
  if (!rank && rank !== 0) {return { kind: 'hidden' };}
  if (suit === undefined) {return { kind: 'hidden' };}
  return { kind: 'standard', rank, suit };
}

export function formatCardToken(token) {
  if (!token) {return '';}
  if (token === 'J0' || token === 'J1') {return token;}
  const [ rank, suit ] = token;
  const suitIcon = {
    C: 'C',
    D: 'D',
    H: 'H',
    S: 'S',
  }[suit] ?? suit;
  return `${rank}${suitIcon}`;
}

export function publicCardToken(card) {
  if (!card || typeof card !== 'object') {return null;}
  if (typeof card.Known === 'string') {return card.Known;}
  return null;
}

export function publicCardToDisplay(card) {
  if (card === 'Hidden') {return { kind: 'hidden' };}
  if (!card) {return { kind: 'hidden' };}
  const token = publicCardToken(card);
  if (token) {
    return parseCardToken(token) ?? { kind: 'hidden' };
  }
  return { kind: 'hidden' };
}
