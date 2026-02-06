const PRIMARY_ACTIONS = new Set([
  'Draw',
  'Pass',
  'PlayPoints',
  'Scuttle',
  'PlayRoyal',
  'PlayJack',
  'PlayJoker',
  'PlayOneOff',
]);

const COUNTER_ACTIONS = new Set([
  'CounterTwo',
  'CounterPass',
]);

const RESOLUTION_ACTIONS = new Set([
  'ResolveThreePick',
  'ResolveFourDiscard',
  'ResolveFiveDiscard',
  'ResolveSevenChoose',
]);

const CHOICE_ORDER = [
  'draw',
  'pass',
  'points',
  'scuttle',
  'royal',
  'jack',
  'joker',
  'oneOff',
  'discard',
  'counterTwo',
  'counterPass',
  'resolveThreePick',
  'resolveFourDiscard',
  'resolveFiveDiscard',
];

function sourceKey(source) {
  if (!source || !source.zone) {return '';}
  if (source.zone === 'reveal') {
    return `reveal:${source.index}`;
  }
  if (source.token !== undefined && source.token !== null) {
    return `${source.zone}:${source.token}`;
  }
  return source.zone;
}

function targetKey(target) {
  if (!target || !target.targetType) {return '';}
  if (target.targetType === 'player') {
    return `player:${target.seat}`;
  }
  if (target.token !== undefined && target.token !== null) {
    return `${target.targetType}:${target.token}`;
  }
  return target.targetType;
}

function dedupeBy(items, getKey) {
  const seen = new Set();
  return items.filter((item) => {
    const key = getKey(item);
    if (!key || seen.has(key)) {return false;}
    seen.add(key);
    return true;
  });
}

function targetFromOneOff(oneOffTarget) {
  if (!oneOffTarget || !oneOffTarget.type) {
    return null;
  }
  switch (oneOffTarget.type) {
    case 'None':
      return null;
    case 'Player':
      return {
        targetType: 'player',
        seat: oneOffTarget.data?.seat,
      };
    case 'Point':
      return {
        targetType: 'point',
        token: oneOffTarget.data?.base,
      };
    case 'Royal':
      return {
        targetType: 'royal',
        token: oneOffTarget.data?.card,
      };
    case 'Jack':
      return {
        targetType: 'jack',
        token: oneOffTarget.data?.card,
      };
    case 'Joker':
      return {
        targetType: 'joker',
        token: oneOffTarget.data?.card,
      };
    default:
      return null;
  }
}

function actionChoiceType(action) {
  if (!action || !action.type) {return null;}
  switch (action.type) {
    case 'Draw':
      return 'draw';
    case 'Pass':
      return 'pass';
    case 'PlayPoints':
      return 'points';
    case 'Scuttle':
      return 'scuttle';
    case 'PlayRoyal':
      return 'royal';
    case 'PlayJack':
      return 'jack';
    case 'PlayJoker':
      return 'joker';
    case 'PlayOneOff':
      return 'oneOff';
    case 'CounterTwo':
      return 'counterTwo';
    case 'CounterPass':
      return 'counterPass';
    case 'ResolveThreePick':
      return 'resolveThreePick';
    case 'ResolveFourDiscard':
      return 'resolveFourDiscard';
    case 'ResolveFiveDiscard':
      return 'resolveFiveDiscard';
    case 'ResolveSevenChoose':
      return sevenChoiceType(action.data?.play);
    default:
      return null;
  }
}

function sevenChoiceType(play) {
  if (!play || !play.type) {return null;}
  switch (play.type) {
    case 'Points':
      return 'points';
    case 'Scuttle':
      return 'scuttle';
    case 'Royal':
      return 'royal';
    case 'Jack':
      return 'jack';
    case 'Joker':
      return 'joker';
    case 'OneOff':
      return 'oneOff';
    case 'Discard':
      return 'discard';
    default:
      return null;
  }
}

function actionTarget(action) {
  if (!action || !action.type) {return null;}
  switch (action.type) {
    case 'Scuttle':
      return {
        targetType: 'point',
        token: action.data?.target_point_base,
      };
    case 'PlayJack':
      return {
        targetType: 'point',
        token: action.data?.target_point_base,
      };
    case 'PlayJoker':
      return {
        targetType: 'royal',
        token: action.data?.target_royal_card,
      };
    case 'PlayOneOff':
      return targetFromOneOff(action.data?.target);
    case 'ResolveSevenChoose':
      return sevenActionTarget(action.data?.play);
    default:
      return null;
  }
}

function sevenActionTarget(play) {
  if (!play || !play.type) {return null;}
  switch (play.type) {
    case 'Scuttle':
      return {
        targetType: 'point',
        token: play.data?.target,
      };
    case 'Jack':
      return {
        targetType: 'point',
        token: play.data?.target,
      };
    case 'Joker':
      return {
        targetType: 'royal',
        token: play.data?.target,
      };
    case 'OneOff':
      return targetFromOneOff(play.data?.target);
    default:
      return null;
  }
}

function sourceMatchesAction(source, action) {
  if (!source) {return false;}
  const actionSource = extractActionSource(action);
  return sourceKey(source) === sourceKey(actionSource);
}

function rankFromToken(token = '') {
  const [ rankChar ] = token;
  if (!rankChar) {return 0;}
  const mapped = {
    A: 1,
    T: 10,
    J: 11,
    Q: 12,
    K: 13,
  }[rankChar];
  if (mapped) {return mapped;}
  const parsed = Number(rankChar);
  return Number.isFinite(parsed) ? parsed : 0;
}

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
              // Seven one-off source is the revealed card at source index and is not logged directly.
              cardToken: null,
              target,
              sourceIndex: Number(sourceIndexToken),
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

function pointsToWinByKings(kings) {
  if (kings >= 3) {return 0;}
  if (kings === 2) {return 5;}
  if (kings === 1) {return 9;}
  return 14;
}

function kingCount(player) {
  return (player?.royals ?? []).filter((stack) => {
    return rankFromToken(stack?.base ?? '') === 13;
  }).length;
}

function pointTotal(player) {
  return (player?.points ?? []).reduce((total, stack) => {
    return total + rankFromToken(stack?.base ?? '');
  }, 0);
}

export function shouldRedirectToCutthroatGame(status) {
  return status === 1;
}

export function isCutthroatGameFinished(status) {
  return status === 2;
}

export function isActionInteractionDisabled(status, actionInFlight) {
  return isCutthroatGameFinished(status) || actionInFlight;
}

export function extractActionSource(action) {
  if (!action || !action.type) {return null;}
  switch (action.type) {
    case 'Draw':
    case 'Pass':
      return { zone: 'deck' };
    case 'PlayPoints':
    case 'Scuttle':
    case 'PlayRoyal':
    case 'PlayOneOff':
    case 'ResolveFourDiscard':
    case 'ResolveFiveDiscard':
      return {
        zone: 'hand',
        token: action.data?.card,
      };
    case 'PlayJack':
      return {
        zone: 'hand',
        token: action.data?.jack,
      };
    case 'PlayJoker':
      return {
        zone: 'hand',
        token: action.data?.joker,
      };
    case 'CounterTwo':
      return {
        zone: 'hand',
        token: action.data?.two_card,
      };
    case 'CounterPass':
      return {
        zone: 'counter',
        token: 'pass',
      };
    case 'ResolveThreePick':
      return {
        zone: 'scrap',
        token: action.data?.card_from_scrap,
      };
    case 'ResolveSevenChoose':
      return {
        zone: 'reveal',
        index: action.data?.source_index,
      };
    default:
      return null;
  }
}

export function deriveMoveChoicesForSource(actions = [], source = null) {
  if (!source) {return [];}
  const choices = actions
    .filter((action) => sourceMatchesAction(source, action))
    .map((action) => actionChoiceType(action))
    .filter(Boolean)
    .map((type) => {
      const orderIndex = CHOICE_ORDER.indexOf(type);
      return {
        type,
        order: orderIndex === -1 ? CHOICE_ORDER.length + 1 : orderIndex,
      };
    });

  return dedupeBy(choices, (choice) => choice.type)
    .sort((a, b) => a.order - b.order)
    .map((choice) => ({ type: choice.type }));
}

export function deriveTargetsForChoice(actions = [], source = null, choice = null) {
  if (!source || !choice) {return [];}
  return dedupeBy(
    actions
      .filter((action) => sourceMatchesAction(source, action))
      .filter((action) => actionChoiceType(action) === choice)
      .map((action) => actionTarget(action))
      .filter(Boolean)
      .map((target) => ({
        ...target,
        key: targetKey(target),
      })),
    (target) => target.key,
  );
}

export function findMatchingAction(actions = [], source = null, choice = null, target = null) {
  if (!source || !choice) {return null;}

  const candidates = actions
    .filter((action) => sourceMatchesAction(source, action))
    .filter((action) => actionChoiceType(action) === choice);

  if (!target) {
    const targetless = candidates.find((action) => !actionTarget(action));
    return targetless ?? candidates[0] ?? null;
  }

  const wantedKey = targetKey(target);
  return candidates.find((action) => {
    const actionTargetValue = actionTarget(action);
    return actionTargetValue && targetKey(actionTargetValue) === wantedKey;
  }) ?? null;
}

export function deriveCutthroatDialogState({
  phaseType = null,
  legalActions = [],
  selectedSource = null,
  selectedChoice = null,
  targets = [],
} = {}) {
  const hasCounterPass = legalActions.some((action) => action?.type === 'CounterPass');
  const counterTwoTokens = dedupeBy(
    legalActions
      .filter((action) => action?.type === 'CounterTwo')
      .map((action) => action?.data?.two_card)
      .filter(Boolean)
      .map((token) => ({ token })),
    (entry) => entry.token,
  ).map((entry) => entry.token);

  const resolveFourTokens = legalActions
    .filter((action) => action?.type === 'ResolveFourDiscard')
    .map((action) => action?.data?.card)
    .filter(Boolean);

  const resolveFiveTokens = legalActions
    .filter((action) => action?.type === 'ResolveFiveDiscard')
    .map((action) => action?.data?.card)
    .filter(Boolean);

  const playerTargetSeats = dedupeBy(
    (targets ?? [])
      .filter((target) => target?.targetType === 'player' && Number.isFinite(target?.seat))
      .map((target) => ({ seat: Number(target.seat) })),
    (target) => `player:${target.seat}`,
  ).map((target) => target.seat);

  const selectedSourceRank = (
    selectedSource?.zone === 'hand'
    && typeof selectedSource?.token === 'string'
  )
    ? rankFromToken(selectedSource.token)
    : null;

  return {
    hasCounterPass,
    counterTwoTokens,
    showCounterDialog: phaseType === 'Countering' && hasCounterPass && counterTwoTokens.length > 0,
    showCannotCounterDialog: phaseType === 'Countering' && hasCounterPass && counterTwoTokens.length === 0,
    resolveFourTokens,
    resolveFiveTokens,
    showResolveFourDialog: phaseType === 'ResolvingFour' && resolveFourTokens.length > 0,
    showResolveFiveDialog: phaseType === 'ResolvingFive' && resolveFiveTokens.length > 0,
    playerTargetSeats,
    showFourPlayerTargetDialog: selectedChoice === 'oneOff' && selectedSourceRank === 4 && playerTargetSeats.length > 0,
  };
}

export function getCutthroatGameResult(status, publicView) {
  if (!isCutthroatGameFinished(status)) {
    return {
      type: 'in_progress',
      seat: null,
    };
  }
  const players = publicView?.players ?? [];
  const winners = players
    .filter((player) => {
      return pointTotal(player) >= pointsToWinByKings(kingCount(player));
    })
    .map((player) => player.seat);

  if (winners.length === 1) {
    const [ winnerSeat ] = winners;
    return {
      type: 'winner',
      seat: winnerSeat,
    };
  }

  return {
    type: 'draw',
    seat: null,
  };
}

export function makeSeatLabel(seat, seats = []) {
  const found = seats.find((entry) => entry.seat === seat);
  const username = found?.username?.trim();
  if (username) {
    return username;
  }
  return `Player ${seat + 1}`;
}

function defaultPointTransition(isLocalSeat) {
  return isLocalSeat ? 'in-below-out-left' : 'in-above-out-below';
}

function defaultRoyalTransition(isLocalSeat) {
  return isLocalSeat ? 'in-below-out-left' : 'in-above-out-below';
}

function isSameSeat(seatA, seatB) {
  return typeof seatA === 'number' && typeof seatB === 'number' && seatA === seatB;
}

export function pointStackTransitionForSeat(lastEvent, seat, mySeat) {
  const isLocalSeat = isSameSeat(seat, mySeat);
  if (!lastEvent?.change) {
    return defaultPointTransition(isLocalSeat);
  }

  switch (lastEvent.change) {
    case 'jack':
    case 'sevenJack':
      return isLocalSeat ? 'slide-above' : 'slide-below';
    case 'resolve':
      switch (lastEvent.oneoff_rank) {
        case 2:
        case 6:
          return isLocalSeat ? 'slide-above' : 'slide-below';
        case 9:
          if (lastEvent.target_type === 'jack') {
            return isLocalSeat ? 'slide-above' : 'slide-below';
          }
          return isLocalSeat ? 'slide-below' : 'slide-above';
        default:
          return defaultPointTransition(isLocalSeat);
      }
    default:
      return defaultPointTransition(isLocalSeat);
  }
}

export function royalStackTransitionForSeat(lastEvent, seat, mySeat) {
  const isLocalSeat = isSameSeat(seat, mySeat);
  if (
    lastEvent?.change === 'resolve'
    && lastEvent?.oneoff_rank === 9
    && (lastEvent?.target_type === 'royal' || lastEvent?.target_type === 'joker')
  ) {
    return isLocalSeat ? 'slide-below' : 'slide-above';
  }
  return defaultRoyalTransition(isLocalSeat);
}

export function scuttledByTokenForPoint(lastEvent, pointBaseToken) {
  if (!lastEvent || lastEvent.change !== 'scuttle') {
    return null;
  }
  if (lastEvent.target_type !== 'point') {
    return null;
  }
  if (lastEvent.target_token !== pointBaseToken) {
    return null;
  }
  return lastEvent.source_token ?? null;
}

export function filterVisibleActions(actions = [], isResolvingSeven = false, selectedRevealIndex = 0) {
  if (!isResolvingSeven) {
    return actions;
  }
  return actions.filter((action) => {
    return action.type === 'ResolveSevenChoose' && action.data?.source_index === selectedRevealIndex;
  });
}

export function groupActions(actions = []) {
  return actions.reduce(
    (groups, action) => {
      if (PRIMARY_ACTIONS.has(action.type)) {
        groups.primary.push(action);
      } else if (COUNTER_ACTIONS.has(action.type)) {
        groups.counter.push(action);
      } else if (RESOLUTION_ACTIONS.has(action.type)) {
        groups.resolution.push(action);
      } else {
        groups.other.push(action);
      }
      return groups;
    },
    {
      primary: [],
      counter: [],
      resolution: [],
      other: [],
    },
  );
}
