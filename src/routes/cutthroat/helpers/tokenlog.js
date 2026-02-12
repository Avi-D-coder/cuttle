const TOKENLOG_CARD_RE = /^(?:[A2-9TJQK][CDHS]|J[01])$/;
const TOKENLOG_SEAT_RE = /^P([0-2])$/;
const TOKENLOG_UNKNOWN_CARD = 'UNKNOWN';

const ACTION_VERBS = new Set([
  'draw',
  'pass',
  'points',
  'scuttle',
  'playRoyal',
  'oneOff',
  'counter',
  'resolve',
  'discard',
]);

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

function isTokenlogCardToken(token) {
  return TOKENLOG_CARD_RE.test(String(token ?? '').toUpperCase());
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

function isSeatToken(token) {
  return typeof token === 'string' && TOKENLOG_SEAT_RE.test(token);
}

function isActionSeatThenVerb(tokens, index) {
  if (!isSeatToken(tokens[index])) {return false;}
  return ACTION_VERBS.has(tokens[index + 1] ?? '');
}

function parseTokenlogOneOffTarget(tokens, startIndex) {
  const token = tokens[startIndex];
  if (!token) {
    return {
      target: { type: 'None' },
      nextIndex: startIndex,
    };
  }

  if (isSeatToken(token)) {
    return {
      target: {
        type: 'Player',
        seat: parseTokenlogSeat(token, startIndex),
      },
      nextIndex: startIndex + 1,
    };
  }

  if (isTokenlogCardToken(token)) {
    return {
      target: {
        type: 'Point',
        token: normalizeTokenlogCard(token, startIndex),
      },
      nextIndex: startIndex + 1,
    };
  }

  return {
    target: { type: 'None' },
    nextIndex: startIndex,
  };
}

function parseGlassesSnapshot(tokens, startIndex) {
  if (!isSeatToken(tokens[startIndex])) {
    return startIndex;
  }

  let cursor = startIndex;
  let groups = 0;
  while (groups < 2 && cursor < tokens.length) {
    if (!isSeatToken(tokens[cursor])) {
      break;
    }
    parseTokenlogSeat(tokens[cursor], cursor);
    cursor += 1;
    while (cursor < tokens.length && isTokenlogCardToken(tokens[cursor])) {
      normalizeTokenlogCard(tokens[cursor], cursor);
      cursor += 1;
    }
    groups += 1;
  }

  if (groups !== 2) {
    return startIndex;
  }

  if (cursor === tokens.length || isActionSeatThenVerb(tokens, cursor)) {
    return cursor;
  }

  return startIndex;
}

function parseTokenlogAction(tokens, startIndex) {
  const actionToken = tokens[startIndex];
  if (!actionToken) {
    throw createTokenlogParseError('Missing action token', startIndex, null);
  }

  switch (actionToken) {
    case 'draw': {
      const maybeCard = tokens[startIndex + 1];
      if (isTokenlogCardToken(maybeCard)) {
        return {
          action: {
            type: 'OTHER',
            cardToken: normalizeTokenlogCard(maybeCard, startIndex + 1),
          },
          nextIndex: startIndex + 2,
        };
      }
      if (String(maybeCard ?? '').toUpperCase() === TOKENLOG_UNKNOWN_CARD) {
        return {
          action: {
            type: 'OTHER',
            cardToken: TOKENLOG_UNKNOWN_CARD,
          },
          nextIndex: startIndex + 2,
        };
      }
      return {
        action: { type: 'OTHER' },
        nextIndex: startIndex + 1,
      };
    }
    case 'pass':
      return {
        action: { type: 'OTHER' },
        nextIndex: startIndex + 1,
      };
    case 'points':
    case 'discard':
      return {
        action: { type: 'OTHER', cardToken: normalizeTokenlogCard(tokens[startIndex + 1], startIndex + 1) },
        nextIndex: startIndex + 2,
      };
    case 'scuttle':
      normalizeTokenlogCard(tokens[startIndex + 1], startIndex + 1);
      normalizeTokenlogCard(tokens[startIndex + 2], startIndex + 2);
      return {
        action: { type: 'OTHER' },
        nextIndex: startIndex + 3,
      };
    case 'playRoyal': {
      const cardToken = normalizeTokenlogCard(tokens[startIndex + 1], startIndex + 1);
      let cursor = startIndex + 2;
      const [ rank ] = cardToken;
      if (rank === 'J' || cardToken === 'J0' || cardToken === 'J1') {
        normalizeTokenlogCard(tokens[cursor], cursor);
        cursor += 1;
      }
      if (rank === '8') {
        cursor = parseGlassesSnapshot(tokens, cursor);
      }
      return {
        action: { type: 'OTHER' },
        nextIndex: cursor,
      };
    }
    case 'oneOff': {
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
    case 'counter':
      return {
        action: {
          type: 'COUNTER_TWO',
          cardToken: normalizeTokenlogCard(tokens[startIndex + 1], startIndex + 1),
        },
        nextIndex: startIndex + 2,
      };
    case 'resolve': {
      if (tokens[startIndex + 1] === 'discard') {
        return {
          action: {
            type: 'OTHER',
            cardToken: normalizeTokenlogCard(tokens[startIndex + 2], startIndex + 2),
          },
          nextIndex: startIndex + 3,
        };
      }
      const maybeCard = tokens[startIndex + 1];
      if (isTokenlogCardToken(maybeCard)) {
        const nextIndex = startIndex + 2;
        if (nextIndex >= tokens.length || isActionSeatThenVerb(tokens, nextIndex)) {
          return {
            action: {
              type: 'OTHER',
              cardToken: normalizeTokenlogCard(maybeCard, startIndex + 1),
            },
            nextIndex,
          };
        }
      }
      return {
        action: {
          type: 'COUNTER_PASS',
        },
        nextIndex: startIndex + 1,
      };
    }
    default:
      throw createTokenlogParseError('Unknown action token', startIndex, actionToken);
  }
}

function normalizePhaseOneOffTarget(target = null) {
  if (!target || typeof target !== 'object') {
    return { type: 'None' };
  }
  switch (target.type) {
    case 'Player':
      return {
        type: 'Player',
        seat: target.data?.seat,
      };
    case 'Point':
      return {
        type: 'Point',
        token: target.data?.base ?? null,
      };
    case 'Royal':
    case 'Jack':
    case 'Joker':
      return {
        type: target.type,
        token: target.data?.card ?? null,
      };
    default:
      return { type: 'None' };
  }
}

function encodeSeatToken(seat) {
  const normalized = Number(seat);
  if (!Number.isInteger(normalized) || normalized < 0 || normalized > 2) {
    throw new Error('Invalid seat for action token encoding');
  }
  return `P${normalized}`;
}

function actionData(action) {
  return action?.data ?? {};
}

function targetData(target) {
  return target?.data ?? {};
}

function normalizeActionCard(cardToken, fieldName) {
  if (!cardToken) {
    throw new Error(`Missing required action card: ${fieldName}`);
  }
  return normalizeTokenlogCard(cardToken, -1);
}

function encodeOneOffTarget(target) {
  if (!target || target.type === 'None') {
    return [];
  }
  if (target.type === 'Player') {
    const seat = target.seat ?? targetData(target).seat;
    return [ encodeSeatToken(seat) ];
  }

  const data = targetData(target);
  const cardToken = target.token ?? data.base ?? data.card ?? null;
  if (!cardToken) {
    throw new Error('Missing one-off target card token');
  }
  return [ normalizeActionCard(cardToken, 'oneoff_target') ];
}

function resolveSevenChosenCard(action, phase) {
  const data = actionData(action);
  const sourceIndex = data.source_index;
  if (!Number.isInteger(sourceIndex) || sourceIndex < 0) {
    throw new Error('ResolveSevenChoose requires source_index');
  }
  const revealed = phase?.type === 'ResolvingSeven' ? (phase?.data?.revealed_cards ?? []) : [];
  const chosen = revealed[sourceIndex];
  if (!chosen) {
    throw new Error('ResolveSevenChoose source_index is not visible in current phase');
  }
  return normalizeActionCard(chosen, 'resolve_seven_revealed');
}

function encodeActionBody(action, phase = null) {
  const data = actionData(action);
  switch (action?.type) {
    case 'Draw':
      return [ 'draw' ];
    case 'Pass':
      return [ 'pass' ];
    case 'PlayPoints':
      return [ 'points', normalizeActionCard(data.card, 'card') ];
    case 'Scuttle':
      return [
        'scuttle',
        normalizeActionCard(data.card, 'card'),
        normalizeActionCard(data.target_point_base, 'target_point_base'),
      ];
    case 'PlayRoyal':
      return [ 'playRoyal', normalizeActionCard(data.card, 'card') ];
    case 'PlayJack':
      return [
        'playRoyal',
        normalizeActionCard(data.jack, 'jack'),
        normalizeActionCard(data.target_point_base, 'target_point_base'),
      ];
    case 'PlayJoker':
      return [
        'playRoyal',
        normalizeActionCard(data.joker, 'joker'),
        normalizeActionCard(data.target_royal_card, 'target_royal_card'),
      ];
    case 'PlayOneOff':
      return [
        'oneOff',
        normalizeActionCard(data.card, 'card'),
        ...encodeOneOffTarget(data.target),
      ];
    case 'CounterTwo':
      return [ 'counter', normalizeActionCard(data.two_card, 'two_card') ];
    case 'CounterPass':
      return [ 'resolve' ];
    case 'ResolveThreePick':
      return [ 'resolve', normalizeActionCard(data.card_from_scrap, 'card_from_scrap') ];
    case 'ResolveFourDiscard':
      return [ 'resolve', 'discard', normalizeActionCard(data.card, 'card') ];
    case 'ResolveFiveDiscard':
      return [ 'discard', normalizeActionCard(data.card, 'card') ];
    case 'ResolveSevenChoose': {
      const chosen = resolveSevenChosenCard(action, phase);
      const play = data.play ?? {};
      const playData = play.data ?? {};
      switch (play.type) {
        case 'Points':
          return [ 'points', chosen ];
        case 'Scuttle':
          return [ 'scuttle', chosen, normalizeActionCard(playData.target, 'seven_scuttle_target') ];
        case 'Royal':
          return [ 'playRoyal', chosen ];
        case 'Jack':
          return [ 'playRoyal', chosen, normalizeActionCard(playData.target, 'seven_jack_target') ];
        case 'Joker':
          return [ 'playRoyal', chosen, normalizeActionCard(playData.target, 'seven_joker_target') ];
        case 'OneOff':
          return [ 'oneOff', chosen, ...encodeOneOffTarget(playData.target) ];
        case 'Discard':
          return [ 'discard', chosen ];
        default:
          throw new Error(`Unsupported seven play type: ${play?.type ?? 'unknown'}`);
      }
    }
    default:
      throw new Error(`Unsupported action type for token encoding: ${action?.type ?? 'unknown'}`);
  }
}

export function encodeActionTokens(action, seat, phase = null) {
  const seatToken = encodeSeatToken(seat);
  const body = encodeActionBody(action, phase);
  return `${seatToken} ${body.join(' ')}`;
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

export function deriveCounterDialogContextFromPhase(phase = null) {
  if (!phase || phase.type !== 'Countering') {return null;}
  const data = phase.data ?? {};
  const oneoff = data.oneoff ?? null;
  if (!oneoff || oneoff.type !== 'PlayOneOff') {return null;}
  return {
    oneOffCardToken: oneoff.data?.card ?? null,
    oneOffTarget: normalizePhaseOneOffTarget(oneoff.data?.target),
    twosPlayed: Array.isArray(data.twos)
      ? data.twos.map((entry) => entry?.card).filter((token) => typeof token === 'string')
      : [],
  };
}

export function deriveCounterDialogContextFromTokenlog(tokenlog = '', maxActions = null) {
  if (!tokenlog || typeof tokenlog !== 'string') {return null;}
  try {
    const parsedActions = parseTokenlogActions(tokenlog);
    if (!Number.isInteger(maxActions) || maxActions < 0) {
      return findActiveCounterChain(parsedActions);
    }
    const actionLimit = Math.min(maxActions, parsedActions.length);
    return findActiveCounterChain(parsedActions.slice(0, actionLimit));
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
