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

function parseTokenlogOneOffTarget(tokens, startIndex, oneOffCardToken = null) {
  const token = tokens[startIndex];
  if (!token) {
    return {
      target: { type: 'None' },
      nextIndex: startIndex,
    };
  }

  if (isSeatToken(token)) {
    const normalizedOneOffCard = typeof oneOffCardToken === 'string'
      ? oneOffCardToken.trim().toUpperCase()
      : '';
    if (!normalizedOneOffCard.startsWith('4')) {
      return {
        target: { type: 'None' },
        nextIndex: startIndex,
      };
    }
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
  const { action, nextIndex } = parseTokenlogActionDetailed(tokens, startIndex);
  return {
    action: mapDetailedActionToContextAction(action),
    nextIndex,
  };
}

function mapDetailedActionToContextAction(action) {
  switch (action?.type) {
    case 'ONEOFF':
      return {
        type: 'ONEOFF',
        cardToken: action.cardToken,
        target: action.target,
      };
    case 'COUNTER_TWO':
      return {
        type: 'COUNTER_TWO',
        cardToken: action.cardToken,
      };
    case 'COUNTER_PASS':
      return {
        type: 'COUNTER_PASS',
      };
    case 'DRAW':
    case 'POINTS':
    case 'RESOLVE_THREE_PICK':
    case 'RESOLVE_FOUR_DISCARD':
    case 'RESOLVE_FIVE_DISCARD':
      return {
        type: 'OTHER',
        cardToken: action.cardToken,
      };
    case 'PASS':
    case 'SCUTTLE':
    case 'PLAY_ROYAL':
    default:
      return {
        type: 'OTHER',
      };
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

function parseTokenlogEnvelope(tokenlog = '') {
  if (typeof tokenlog !== 'string') {
    throw createTokenlogParseError('Tokenlog must be a string', 0, null);
  }
  const trimmed = tokenlog.trim();
  if (!trimmed) {
    return {
      dealer: null,
      tokens: [],
      actionCursor: 0,
    };
  }

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
  const dealer = parseTokenlogSeat(tokens[cursor], cursor);
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

  return {
    dealer,
    tokens,
    actionCursor: cursor,
  };
}

export function parseTokenlogActions(tokenlog = '') {
  const { actions } = parseTokenlogActionStream(tokenlog, parseTokenlogAction);
  return actions;
}

const ONE_OFF_EFFECTS = {
  A: 'Scrap all points',
  '2': 'Scrap target Royal or Glasses eight',
  '3': 'Choose 1 (non-three) card in the Scrap and put it to your hand',
  '4': 'Your opponent discards two cards of their choice from their hand',
  '5': 'Discard 1 card, and draw up to 3',
  '6': 'Scrap all Royals and Glasses eights',
  '7': 'Play one of the top two cards of the deck and put the other back (both are revealed)',
  '8': 'Your opponent plays with an open hand (their cards are revealed to you)',
  '9': 'Return target card to its controller\'s hand. They can\'t play it next turn',
  T: 'No effect',
};

function historyRankSymbol(rankToken) {
  return {
    A: 'A',
    T: '10',
    J: 'J',
    Q: 'Q',
    K: 'K',
  }[rankToken] ?? rankToken;
}

function cardTokenToHistoryName(token) {
  if (!token || token === TOKENLOG_UNKNOWN_CARD) {return 'Unknown card';}
  if (token === 'J0') {return 'Joker 0';}
  if (token === 'J1') {return 'Joker 1';}
  const normalized = String(token)
    .trim()
    .toUpperCase();
  if (!TOKENLOG_CARD_RE.test(normalized)) {return 'Unknown card';}
  const [ rank, suit ] = normalized;
  const suitSymbol = {
    C: '♣️',
    D: '♦️',
    H: '♥️',
    S: '♠️',
  }[suit] ?? '';
  return `${historyRankSymbol(rank)}${suitSymbol}`;
}

function cardRankToken(cardToken = '') {
  if (!cardToken || cardToken === TOKENLOG_UNKNOWN_CARD) {return '';}
  const normalized = String(cardToken)
    .trim()
    .toUpperCase();
  if (normalized === 'J0' || normalized === 'J1') {return normalized;}
  return normalized[0] ?? '';
}

function seatNameForHistory(seat, seatNames = {}) {
  if (!Number.isInteger(seat) || seat < 0) {return 'Player';}
  if (Array.isArray(seatNames) && typeof seatNames[seat] === 'string' && seatNames[seat]) {
    return seatNames[seat];
  }
  if (seatNames && typeof seatNames === 'object') {
    const fromNumKey = seatNames[seat];
    if (typeof fromNumKey === 'string' && fromNumKey) {return fromNumKey;}
    const fromSeatKey = seatNames[`P${seat}`];
    if (typeof fromSeatKey === 'string' && fromSeatKey) {return fromSeatKey;}
  }
  return `Player ${seat + 1}`;
}

function targetTextForOneOff(target, seatNames) {
  if (!target || target.type === 'None') {return '';}
  if (target.type === 'Player') {
    return `, targeting ${seatNameForHistory(target.seat, seatNames)}`;
  }
  const token = target.token ?? null;
  if (!token) {return '';}
  return `, targeting the ${cardTokenToHistoryName(token)}`;
}

function oneOffResolveLine(action, seatNames, didResolve) {
  const cardName = cardTokenToHistoryName(action.cardToken);
  if (!didResolve) {
    return `The ${cardName} is countered, and all cards played this turn are scrapped.`;
  }

  const rank = cardRankToken(action.cardToken);
  const targetSeat = action.target?.type === 'Player' ? action.target.seat : null;
  const targetName = Number.isInteger(targetSeat) ? seatNameForHistory(targetSeat, seatNames) : 'a player';
  const targetCardName = action.target?.token ? cardTokenToHistoryName(action.target.token) : 'target card';
  switch (rank) {
    case 'A':
      return `The ${cardName} one-off resolves; all point cards are scrapped.`;
    case '2':
      return `The ${cardName} resolves; the ${targetCardName} is scrapped.`;
    case '3':
      return `The ${cardName} one-off resolves; ${seatNameForHistory(action.seat, seatNames)} will draw one card of their choice from the Scrap pile.`;
    case '4':
      return `The ${cardName} one-off resolves; ${targetName} must discard two cards.`;
    case '5':
      return `The ${cardName} one-off resolves; ${seatNameForHistory(action.seat, seatNames)} must discard 1 card, and will draw up to 3.`;
    case '6':
      return `The ${cardName} one-off resolves; all Royals and Glasses are scrapped.`;
    case '7':
      return `The ${cardName} one-off resolves; they will play one card from the top two in the deck.`;
    case '9':
      return `The ${cardName} one-off resolves, returning the ${targetCardName} to its controller's hand. It cannot be played next turn.`;
    default:
      return `The ${cardName} one-off resolves.`;
  }
}

function parseTokenlogActionDetailed(tokens, startIndex) {
  const actionToken = tokens[startIndex];
  if (!actionToken) {
    throw createTokenlogParseError('Missing action token', startIndex, null);
  }

  switch (actionToken) {
    case 'draw': {
      const maybeCard = tokens[startIndex + 1];
      if (isTokenlogCardToken(maybeCard) || String(maybeCard ?? '').toUpperCase() === TOKENLOG_UNKNOWN_CARD) {
        return {
          action: {
            type: 'DRAW',
            cardToken: String(maybeCard).toUpperCase(),
          },
          nextIndex: startIndex + 2,
        };
      }
      return {
        action: { type: 'DRAW' },
        nextIndex: startIndex + 1,
      };
    }
    case 'pass':
      return {
        action: { type: 'PASS' },
        nextIndex: startIndex + 1,
      };
    case 'points':
      return {
        action: {
          type: 'POINTS',
          cardToken: normalizeTokenlogCard(tokens[startIndex + 1], startIndex + 1),
        },
        nextIndex: startIndex + 2,
      };
    case 'scuttle':
      return {
        action: {
          type: 'SCUTTLE',
          cardToken: normalizeTokenlogCard(tokens[startIndex + 1], startIndex + 1),
          targetCardToken: normalizeTokenlogCard(tokens[startIndex + 2], startIndex + 2),
        },
        nextIndex: startIndex + 3,
      };
    case 'playRoyal': {
      const cardToken = normalizeTokenlogCard(tokens[startIndex + 1], startIndex + 1);
      let cursor = startIndex + 2;
      let targetCardToken = null;
      const rank = cardRankToken(cardToken);
      if (rank === 'J' || cardToken === 'J0' || cardToken === 'J1') {
        targetCardToken = normalizeTokenlogCard(tokens[cursor], cursor);
        cursor += 1;
      }
      if (rank === '8') {
        cursor = parseGlassesSnapshot(tokens, cursor);
      }
      return {
        action: {
          type: 'PLAY_ROYAL',
          cardToken,
          targetCardToken,
        },
        nextIndex: cursor,
      };
    }
    case 'oneOff': {
      const cardToken = normalizeTokenlogCard(tokens[startIndex + 1], startIndex + 1);
      const { target, nextIndex } = parseTokenlogOneOffTarget(tokens, startIndex + 2, cardToken);
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
            type: 'RESOLVE_FOUR_DISCARD',
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
              type: 'RESOLVE_THREE_PICK',
              cardToken: normalizeTokenlogCard(maybeCard, startIndex + 1),
            },
            nextIndex,
          };
        }
      }
      return {
        action: { type: 'COUNTER_PASS' },
        nextIndex: startIndex + 1,
      };
    }
    case 'discard':
      return {
        action: {
          type: 'RESOLVE_FIVE_DISCARD',
          cardToken: normalizeTokenlogCard(tokens[startIndex + 1], startIndex + 1),
        },
        nextIndex: startIndex + 2,
      };
    default:
      throw createTokenlogParseError('Unknown action token', startIndex, actionToken);
  }
}

function parseTokenlogActionsForHistory(tokenlog = '') {
  return parseTokenlogActionStream(tokenlog, parseTokenlogActionDetailed);
}

function parseTokenlogActionStream(tokenlog = '', parseAction) {
  const { dealer, tokens, actionCursor } = parseTokenlogEnvelope(tokenlog);
  if (tokens.length === 0) {
    return {
      dealer: null,
      actions: [],
    };
  }
  const actions = [];
  let cursor = actionCursor;
  while (cursor < tokens.length) {
    const seat = parseTokenlogSeat(tokens[cursor], cursor);
    cursor += 1;
    const { action, nextIndex } = parseAction(tokens, cursor);
    actions.push({
      ...action,
      seat,
    });
    cursor = nextIndex;
  }
  return {
    dealer,
    actions,
  };
}

export function findActiveCounterChain(parsedActions = []) {
  if (!Array.isArray(parsedActions) || parsedActions.length === 0) {return null;}
  const twosPlayed = [];
  let triggeringSeat = null;
  let index = parsedActions.length - 1;

  while (index >= 0) {
    const action = parsedActions[index];
    if (action?.type === 'COUNTER_TWO') {
      if (!action.cardToken) {return null;}
      if (!Number.isInteger(triggeringSeat) && Number.isInteger(action.seat)) {
        triggeringSeat = action.seat;
      }
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
    oneOffSeat: Number.isInteger(oneOffAction.seat) ? oneOffAction.seat : null,
    triggeringSeat: Number.isInteger(triggeringSeat)
      ? triggeringSeat
      : (Number.isInteger(oneOffAction.seat) ? oneOffAction.seat : null),
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
  const twos = Array.isArray(data.twos) ? data.twos : [];
  const lastTwo = twos.length > 0 ? twos[twos.length - 1] : null;
  const oneOffSeat = Number.isInteger(data.base_player) ? data.base_player : null;
  return {
    oneOffSeat,
    triggeringSeat: Number.isInteger(lastTwo?.seat) ? lastTwo.seat : oneOffSeat,
    oneOffCardToken: oneoff.data?.card ?? null,
    oneOffTarget: normalizePhaseOneOffTarget(oneoff.data?.target),
    twosPlayed: twos.map((entry) => entry?.card).filter((token) => typeof token === 'string'),
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

export function deriveLatestOneOffContextFromTokenlog(tokenlog = '', maxActions = null) {
  if (!tokenlog || typeof tokenlog !== 'string') {return null;}
  try {
    const parsedActions = parseTokenlogActions(tokenlog);
    const limited = Number.isInteger(maxActions) && maxActions >= 0
      ? parsedActions.slice(0, Math.min(maxActions, parsedActions.length))
      : parsedActions;

    for (let index = limited.length - 1; index >= 0; index -= 1) {
      const action = limited[index];
      if (action?.type !== 'ONEOFF' || !action.cardToken) {continue;}
      return {
        oneOffSeat: Number.isInteger(action.seat) ? action.seat : null,
        oneOffCardToken: action.cardToken,
        oneOffTarget: action.target ?? { type: 'None' },
      };
    }
    return null;
  } catch (_) {
    return null;
  }
}

export function formatTokenlogForHistory(tokenlog = '', options = {}) {
  if (typeof tokenlog !== 'string') {return [];}
  const trimmed = tokenlog.trim();
  if (!trimmed) {return [];}

  const seatNames = options?.seatNames ?? {};
  const maxActions = options?.maxActions;

  try {
    const parsed = parseTokenlogActionsForHistory(trimmed);
    const actionCount = Number.isInteger(maxActions) && maxActions >= 0
      ? Math.min(maxActions, parsed.actions.length)
      : parsed.actions.length;
    const actions = parsed.actions.slice(0, actionCount);
    const lines = [];

    if (Number.isInteger(parsed.dealer)) {
      const dealerName = seatNameForHistory(parsed.dealer, seatNames);
      const firstSeat = (parsed.dealer + 1) % 3;
      lines.push(`${dealerName} dealt; ${seatNameForHistory(firstSeat, seatNames)} will go first`);
    }

    let pendingSevenSeat = null;
    let pendingOneOffResult = null;
    for (let index = 0; index < actions.length; index += 1) {
      const action = actions[index];
      const actor = seatNameForHistory(action.seat, seatNames);
      const actionFromSeven = Number.isInteger(pendingSevenSeat) && pendingSevenSeat === action.seat;

      switch (action.type) {
        case 'DRAW':
          lines.push(`${actor} drew a card.`);
          break;
        case 'PASS':
          lines.push(`${actor} passed.`);
          break;
        case 'POINTS':
          if (actionFromSeven) {
            lines.push(`${actor} played the ${cardTokenToHistoryName(action.cardToken)} from the top of the deck for points.`);
          } else {
            lines.push(`${actor} played the ${cardTokenToHistoryName(action.cardToken)} for points.`);
          }
          break;
        case 'SCUTTLE':
          if (actionFromSeven) {
            lines.push(
              `${actor} scuttled the ${cardTokenToHistoryName(action.targetCardToken)} with the ${cardTokenToHistoryName(action.cardToken)} from the top of the deck.`,
            );
          } else {
            lines.push(
              `${actor} scuttled the ${cardTokenToHistoryName(action.targetCardToken)} with the ${cardTokenToHistoryName(action.cardToken)}.`,
            );
          }
          break;
        case 'PLAY_ROYAL': {
          const rank = cardRankToken(action.cardToken);
          if (rank === 'J') {
            if (actionFromSeven) {
              lines.push(
                `${actor} stole the ${cardTokenToHistoryName(action.targetCardToken)} with the ${cardTokenToHistoryName(action.cardToken)} from the top of the deck.`,
              );
            } else {
              lines.push(
                `${actor} stole the ${cardTokenToHistoryName(action.targetCardToken)} with the ${cardTokenToHistoryName(action.cardToken)}.`,
              );
            }
          } else if (action.cardToken === 'J0' || action.cardToken === 'J1') {
            lines.push(
              `${actor} played the ${cardTokenToHistoryName(action.cardToken)} on the ${cardTokenToHistoryName(action.targetCardToken)}.`,
            );
          } else if (rank === '8') {
            lines.push(
              `${actor} played the ${cardTokenToHistoryName(action.cardToken)}${actionFromSeven ? ' from the top of the deck' : ''} as a glasses eight.`,
            );
          } else {
            lines.push(
              `${actor} played the ${cardTokenToHistoryName(action.cardToken)}${actionFromSeven ? ' from the top of the deck' : ''}.`,
            );
          }
          break;
        }
        case 'ONEOFF': {
          const effectText = ONE_OFF_EFFECTS[cardRankToken(action.cardToken)] ?? 'Resolve a one-off effect';
          const oneOffTarget = targetTextForOneOff(action.target, seatNames);
          if (actionFromSeven) {
            lines.push(
              `${actor} played the ${cardTokenToHistoryName(action.cardToken)} from the top of the deck as a one-off to ${effectText}${oneOffTarget}.`,
            );
          } else {
            lines.push(
              `${actor} played the ${cardTokenToHistoryName(action.cardToken)} as a one-off to ${effectText}${oneOffTarget}.`,
            );
          }

          let twoCount = 0;
          let cursor = index + 1;
          while (cursor < actions.length && (actions[cursor].type === 'COUNTER_TWO' || actions[cursor].type === 'COUNTER_PASS')) {
            if (actions[cursor].type === 'COUNTER_TWO') {
              twoCount += 1;
            }
            cursor += 1;
          }
          const didResolve = (twoCount % 2) === 0;
          pendingOneOffResult = {
            flushAfterIndex: cursor - 1,
            line: oneOffResolveLine(action, seatNames, didResolve),
            didResolve,
            rank: cardRankToken(action.cardToken),
            seat: action.seat,
          };
          break;
        }
        case 'COUNTER_TWO':
          lines.push(`${actor} played the ${cardTokenToHistoryName(action.cardToken)} to counter.`);
          break;
        case 'COUNTER_PASS':
          break;
        case 'RESOLVE_THREE_PICK':
          lines.push(`${actor} took the ${cardTokenToHistoryName(action.cardToken)} from the Scrap pile to their hand.`);
          break;
        case 'RESOLVE_FOUR_DISCARD':
          if (
            index + 1 < actions.length
            && actions[index + 1].type === 'RESOLVE_FOUR_DISCARD'
            && actions[index + 1].seat === action.seat
          ) {
            lines.push(
              `${actor} discarded the ${cardTokenToHistoryName(action.cardToken)} and the ${
                cardTokenToHistoryName(actions[index + 1].cardToken)
              }.`,
            );
            index += 1;
          } else {
            lines.push(`${actor} discarded the ${cardTokenToHistoryName(action.cardToken)}.`);
          }
          break;
        case 'RESOLVE_FIVE_DISCARD':
          lines.push(`${actor} discarded the ${cardTokenToHistoryName(action.cardToken)}.`);
          break;
        default:
          break;
      }

      if (actionFromSeven && action.type !== 'COUNTER_TWO' && action.type !== 'COUNTER_PASS') {
        pendingSevenSeat = null;
      }

      if (pendingOneOffResult && index >= pendingOneOffResult.flushAfterIndex) {
        lines.push(pendingOneOffResult.line);
        if (pendingOneOffResult.didResolve && pendingOneOffResult.rank === '7') {
          pendingSevenSeat = pendingOneOffResult.seat;
        }
        pendingOneOffResult = null;
      }
    }

    return lines;
  } catch (_) {
    return [];
  }
}
