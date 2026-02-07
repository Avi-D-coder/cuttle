const TOKENLOG_CARD_RE = /^(?:[A2-9TJQK][CDHS]|J[01])$/;
const TOKENLOG_SEAT_RE = /^P([0-2])$/;

function createTokenlogParseError(message, index, token = null) {
  const error = new Error(message);
  error.name = 'TokenlogParseError';
  error.code = 'TOKENLOG_PARSE_ERROR';
  error.index = index;
  error.token = token;
  return error;
}

function normalizeTokenlogCard(token, index) {
  if (typeof token !== 'string') {
    throw createTokenlogParseError('Expected card token string', index, token ?? null);
  }
  const normalized = token.trim().toUpperCase();
  if (!TOKENLOG_CARD_RE.test(normalized)) {
    throw createTokenlogParseError('Invalid card token', index, token);
  }
  return normalized;
}

function parseTokenlogSeat(token, index) {
  if (typeof token !== 'string') {
    throw createTokenlogParseError('Expected seat token string', index, token ?? null);
  }
  const match = token.match(TOKENLOG_SEAT_RE);
  if (!match) {
    throw createTokenlogParseError('Invalid seat token', index, token);
  }
  return Number(match[1]);
}

function parseTokenlogOneOffTarget(tokens, startIndex) {
  const token = tokens[startIndex];
  if (!token || !token.startsWith('TGT_')) {
    return {
      target: {
        type: 'None',
      },
      nextIndex: startIndex,
    };
  }

  if (token === 'TGT_P') {
    const seatToken = tokens[startIndex + 1];
    return {
      target: {
        type: 'Player',
        seat: parseTokenlogSeat(seatToken, startIndex + 1),
      },
      nextIndex: startIndex + 2,
    };
  }

  const cardTargetType = {
    TGT_POINT: 'Point',
    TGT_ROYAL: 'Royal',
    TGT_JACK: 'Jack',
    TGT_JOKER: 'Joker',
  }[token];

  if (cardTargetType) {
    return {
      target: {
        type: cardTargetType,
        token: normalizeTokenlogCard(tokens[startIndex + 1], startIndex + 1),
      },
      nextIndex: startIndex + 2,
    };
  }

  throw createTokenlogParseError('Unknown one-off target token', startIndex, token);
}

function parseTokenlogAction(tokens, startIndex) {
  const actionToken = tokens[startIndex];
  if (!actionToken) {
    throw createTokenlogParseError('Missing action token', startIndex, null);
  }

  switch (actionToken) {
    case 'MT_DRAW':
    case 'MT_PASS':
      return {
        action: { type: 'OTHER' },
        nextIndex: startIndex + 1,
      };
    case 'MT_POINTS':
    case 'MT_ROYAL':
    case 'MT_R3_PICK':
    case 'MT_R4_DISCARD':
    case 'MT_R5_DISCARD':
      return {
        action: { type: 'OTHER', cardToken: normalizeTokenlogCard(tokens[startIndex + 1], startIndex + 1) },
        nextIndex: startIndex + 2,
      };
    case 'MT_SCUTTLE':
    case 'MT_JACK':
    case 'MT_JOKER': {
      const tgtKeyword = tokens[startIndex + 2];
      if (tgtKeyword !== 'TGT') {
        throw createTokenlogParseError('Expected TGT marker', startIndex + 2, tgtKeyword ?? null);
      }
      normalizeTokenlogCard(tokens[startIndex + 1], startIndex + 1);
      normalizeTokenlogCard(tokens[startIndex + 3], startIndex + 3);
      return {
        action: { type: 'OTHER' },
        nextIndex: startIndex + 4,
      };
    }
    case 'MT_ONEOFF': {
      const cardToken = normalizeTokenlogCard(tokens[startIndex + 1], startIndex + 1);
      const { target, nextIndex } = parseTokenlogOneOffTarget(tokens, startIndex + 2);
      return {
        action: {
          type: 'ONEOFF',
          cardToken,
          target,
        },
        nextIndex,
      };
    }
    case 'MT_C2':
      return {
        action: {
          type: 'COUNTER_TWO',
          cardToken: normalizeTokenlogCard(tokens[startIndex + 1], startIndex + 1),
        },
        nextIndex: startIndex + 2,
      };
    case 'MT_CPASS':
      return {
        action: {
          type: 'COUNTER_PASS',
        },
        nextIndex: startIndex + 1,
      };
    case 'MT_R7': {
      if (tokens[startIndex + 1] !== 'SRC') {
        throw createTokenlogParseError('Expected SRC marker', startIndex + 1, tokens[startIndex + 1] ?? null);
      }
      const sourceIndexToken = tokens[startIndex + 2];
      if (!/^\d+$/.test(String(sourceIndexToken ?? ''))) {
        throw createTokenlogParseError('Invalid seven source index', startIndex + 2, sourceIndexToken ?? null);
      }
      if (tokens[startIndex + 3] !== 'AS') {
        throw createTokenlogParseError('Expected AS marker', startIndex + 3, tokens[startIndex + 3] ?? null);
      }
      const playToken = tokens[startIndex + 4];
      if (!playToken) {
        throw createTokenlogParseError('Missing seven play token', startIndex + 4, null);
      }
      switch (playToken) {
        case 'POINTS':
        case 'ROYAL':
        case 'DISCARD':
          return {
            action: { type: 'OTHER' },
            nextIndex: startIndex + 5,
          };
        case 'SCUTTLE':
        case 'JACK':
        case 'JOKER':
          normalizeTokenlogCard(tokens[startIndex + 5], startIndex + 5);
          return {
            action: { type: 'OTHER' },
            nextIndex: startIndex + 6,
          };
        case 'ONEOFF': {
          const { target, nextIndex } = parseTokenlogOneOffTarget(tokens, startIndex + 5);
          return {
            action: {
              type: 'ONEOFF',
              cardToken: null,
              sourceIndex: Number(sourceIndexToken),
              target,
            },
            nextIndex,
          };
        }
        default:
          throw createTokenlogParseError('Unknown seven play token', startIndex + 4, playToken);
      }
    }
    default:
      throw createTokenlogParseError('Unknown action token', startIndex, actionToken);
  }
}

export function parseTokenlogActions(tokenlog = '') {
  if (typeof tokenlog !== 'string') {
    throw createTokenlogParseError('Tokenlog must be a string', 0, null);
  }
  const trimmed = tokenlog.trim();
  if (!trimmed) {return [];}

  const tokens = trimmed.split(/\s+/);
  let cursor = 0;
  if (tokens[cursor] !== 'V1') {
    throw createTokenlogParseError('Expected tokenlog version V1', cursor, tokens[cursor] ?? null);
  }
  cursor += 1;
  if (tokens[cursor] !== 'CUTTHROAT3P') {
    throw createTokenlogParseError('Expected CUTTHROAT3P mode', cursor, tokens[cursor] ?? null);
  }
  cursor += 1;
  if (tokens[cursor] !== 'DEALER') {
    throw createTokenlogParseError('Expected DEALER marker', cursor, tokens[cursor] ?? null);
  }
  cursor += 1;
  parseTokenlogSeat(tokens[cursor], cursor);
  cursor += 1;
  if (tokens[cursor] !== 'DECK') {
    throw createTokenlogParseError('Expected DECK marker', cursor, tokens[cursor] ?? null);
  }
  cursor += 1;
  while (cursor < tokens.length && tokens[cursor] !== 'ENDDECK') {
    normalizeTokenlogCard(tokens[cursor], cursor);
    cursor += 1;
  }
  if (tokens[cursor] !== 'ENDDECK') {
    throw createTokenlogParseError('Missing ENDDECK marker', cursor, tokens[cursor] ?? null);
  }
  cursor += 1;

  const parsedActions = [];
  while (cursor < tokens.length) {
    const seat = parseTokenlogSeat(tokens[cursor], cursor);
    cursor += 1;
    const { action, nextIndex } = parseTokenlogAction(tokens, cursor);
    parsedActions.push({
      ...action,
      seat,
    });
    cursor = nextIndex;
  }
  return parsedActions;
}

export function findActiveCounterChain(parsedActions = []) {
  if (!Array.isArray(parsedActions) || parsedActions.length === 0) {return null;}
  const twosPlayed = [];
  let index = parsedActions.length - 1;

  while (index >= 0) {
    const action = parsedActions[index];
    if (action?.type === 'COUNTER_TWO') {
      if (!action.cardToken) {return null;}
      twosPlayed.unshift(action.cardToken);
      index -= 1;
      continue;
    }
    if (action?.type === 'COUNTER_PASS') {
      index -= 1;
      continue;
    }
    break;
  }

  if (index < 0) {return null;}
  const oneOffAction = parsedActions[index];
  if (oneOffAction?.type !== 'ONEOFF' || !oneOffAction.cardToken) {
    return null;
  }

  return {
    oneOffCardToken: oneOffAction.cardToken,
    oneOffTarget: oneOffAction.target ?? { type: 'None' },
    twosPlayed,
  };
}

export function deriveCounterDialogContextFromTokenlog(tokenlog = '') {
  if (!tokenlog || typeof tokenlog !== 'string') {return null;}
  try {
    const parsedActions = parseTokenlogActions(tokenlog);
    return findActiveCounterChain(parsedActions);
  } catch (_) {
    return null;
  }
}

export function formatTokenlogForHistory(tokenlog = '') {
  if (typeof tokenlog !== 'string') {return [];}
  const trimmed = tokenlog.trim();
  if (!trimmed) {return [];}
  return [ trimmed ];
}
