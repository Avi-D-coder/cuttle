const CARD_TOKEN_RE = /^(?:[A2-9TJQK][CDHS]|J[01])$/;
const SEAT_TOKEN_RE = /^P([0-2])$/;

const PRIMARY_CHOICES = new Set([ 'draw', 'pass', 'points', 'scuttle', 'royal', 'jack', 'joker', 'oneOff', 'stalemateRequest' ]);
const COUNTER_CHOICES = new Set([ 'counterTwo', 'counterPass' ]);
const RESOLUTION_CHOICES = new Set([ 'resolveThreePick', 'resolveFourDiscard', 'resolveFiveDiscard', 'discard', 'stalemateAccept', 'stalemateReject' ]);

const CHOICE_ORDER = [
  'draw',
  'pass',
  'stalemateRequest',
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
  'stalemateAccept',
  'stalemateReject',
];

function normalizeVerb(verb = '') {
  return String(verb ?? '').trim()
    .toLowerCase();
}

function isStalemateRequestVerb(verb = '') {
  const normalized = normalizeVerb(verb);
  return normalized === 'stalemate-propose'
    || normalized === 'stalemate_propose'
    || normalized === 'stalemate_request'
    || normalized === 'requeststalemate'
    || normalized === 'stalemateoffer'
    || normalized === 'offerstalemate'
    || normalized === 'drawoffer'
    || normalized === 'offerdraw';
}

function isStalemateAcceptVerb(verb = '') {
  const normalized = normalizeVerb(verb);
  return normalized === 'stalemate-accept'
    || normalized === 'stalemate_accept'
    || normalized === 'stalemateaccept'
    || normalized === 'acceptstalemate'
    || normalized === 'acceptdraw'
    || normalized === 'drawaccept';
}

function isStalemateRejectVerb(verb = '') {
  const normalized = normalizeVerb(verb);
  return normalized === 'stalemate-reject'
    || normalized === 'stalemate_reject'
    || normalized === 'stalematereject'
    || normalized === 'rejectstalemate'
    || normalized === 'rejectdraw'
    || normalized === 'drawreject';
}

function sourceKey(source) {
  if (!source || !source.zone) {return '';}
  if (source.zone === 'reveal') {
    return `reveal:${source.token ?? ''}`;
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

function parseSeat(seatToken) {
  const match = String(seatToken ?? '').match(SEAT_TOKEN_RE);
  if (!match) {return null;}
  return Number(match[1]);
}

function isCardToken(token) {
  return CARD_TOKEN_RE.test(String(token ?? '').toUpperCase());
}

function normalizeCardToken(token) {
  const normalized = String(token ?? '').toUpperCase();
  return isCardToken(normalized) ? normalized : null;
}

function parseLegalActionToken(actionToken = '') {
  if (typeof actionToken !== 'string') {return null;}
  const parts = actionToken
    .trim()
    .split(/\s+/)
    .filter(Boolean);
  if (parts.length < 2) {return null;}
  const seat = parseSeat(parts[0]);
  if (seat === null) {return null;}
  return {
    seat,
    verb: parts[1],
    args: parts.slice(2),
    token: actionToken,
  };
}

function playRoyalChoiceForCard(cardToken) {
  const card = normalizeCardToken(cardToken);
  if (!card) {return null;}
  if (card === 'J0' || card === 'J1') {return 'joker';}
  return card[0] === 'J' ? 'jack' : 'royal';
}

function choiceFromParsed(parsed, phaseType = null) {
  if (!parsed) {return null;}
  const { verb, args } = parsed;
  if (isStalemateRequestVerb(verb)) {return 'stalemateRequest';}
  if (isStalemateAcceptVerb(verb)) {return 'stalemateAccept';}
  if (isStalemateRejectVerb(verb)) {return 'stalemateReject';}
  if (verb === 'draw') {return 'draw';}
  if (verb === 'pass') {return 'pass';}
  if (verb === 'points') {return 'points';}
  if (verb === 'scuttle') {return 'scuttle';}
  if (verb === 'oneOff') {return 'oneOff';}
  if (verb === 'counter') {return 'counterTwo';}
  if (verb === 'playRoyal') {return playRoyalChoiceForCard(args[0]);}
  if (verb === 'resolve') {
    if (args.length === 0) {return 'counterPass';}
    if (args[0] === 'discard' && normalizeCardToken(args[1])) {return 'resolveFourDiscard';}
    if (normalizeCardToken(args[0])) {return 'resolveThreePick';}
    return null;
  }
  if (verb === 'discard') {
    if (!normalizeCardToken(args[0])) {return null;}
    return phaseType === 'ResolvingSeven' ? 'discard' : 'resolveFiveDiscard';
  }
  return null;
}

function sourceFromParsed(parsed, phaseType = null) {
  if (!parsed) {return null;}
  const choice = choiceFromParsed(parsed, phaseType);
  if (!choice) {return null;}

  if (choice === 'draw' || choice === 'pass') {
    return { zone: 'deck' };
  }
  if (choice === 'counterPass') {
    return { zone: 'counter', token: 'pass' };
  }
  if (choice === 'stalemateRequest') {
    return { zone: 'stalemate', token: 'request' };
  }
  if (choice === 'stalemateAccept') {
    return { zone: 'stalemate', token: 'accept' };
  }
  if (choice === 'stalemateReject') {
    return { zone: 'stalemate', token: 'reject' };
  }
  if (choice === 'counterTwo') {
    return { zone: 'hand', token: normalizeCardToken(parsed.args[0]) };
  }
  if (choice === 'resolveThreePick') {
    return { zone: 'scrap', token: normalizeCardToken(parsed.args[0]) };
  }
  if (choice === 'resolveFourDiscard') {
    return { zone: 'hand', token: normalizeCardToken(parsed.args[1]) };
  }
  if (choice === 'resolveFiveDiscard') {
    return { zone: 'hand', token: normalizeCardToken(parsed.args[0]) };
  }

  if (phaseType === 'ResolvingSeven') {
    return { zone: 'reveal', token: normalizeCardToken(parsed.args[0]) };
  }

  return { zone: 'hand', token: normalizeCardToken(parsed.args[0]) };
}

function targetFromParsed(parsed, phaseType = null) {
  if (!parsed) {return null;}
  const choice = choiceFromParsed(parsed, phaseType);
  if (!choice) {return null;}

  if (choice === 'scuttle') {
    return { targetType: 'point', token: normalizeCardToken(parsed.args[1]) };
  }
  if (choice === 'jack') {
    return { targetType: 'point', token: normalizeCardToken(parsed.args[1]) };
  }
  if (choice === 'joker') {
    return { targetType: 'royal', token: normalizeCardToken(parsed.args[1]) };
  }
  if (choice !== 'oneOff') {
    return null;
  }

  const [ , raw ] = parsed.args;
  const seat = parseSeat(raw);
  if (seat !== null) {
    return { targetType: 'player', seat };
  }
  const token = normalizeCardToken(raw);
  if (!token) {
    return null;
  }
  return { targetType: 'card', token };
}

function sourceMatchesParsed(source, parsed, phaseType = null) {
  if (!source || !parsed) {return false;}
  const parsedSource = sourceFromParsed(parsed, phaseType);
  return sourceKey(source) === sourceKey(parsedSource);
}

function rankFromToken(token = '') {
  const [ rankChar ] = token;
  if (!rankChar) {return 0;}
  const mapped = { A: 1, T: 10, J: 11, Q: 12, K: 13 }[rankChar];
  if (mapped) {return mapped;}
  const parsed = Number(rankChar);
  return Number.isFinite(parsed) ? parsed : 0;
}

export function deriveFallbackChoiceTypesForSelectedCard(source = null, selectedCard = null) {
  if (source?.zone !== 'hand') {return [];}
  if (!selectedCard || typeof selectedCard !== 'object') {return [];}

  if (selectedCard.kind === 'joker') {
    return [ 'joker' ];
  }

  if (selectedCard.kind !== 'standard') {return [];}

  switch (selectedCard.rank) {
    case 1:
    case 2:
    case 3:
    case 4:
    case 5:
    case 6:
    case 7:
    case 9:
      return [ 'points', 'scuttle', 'oneOff' ];
    case 8:
      return [ 'points', 'scuttle', 'royal' ];
    case 10:
      return [ 'points', 'scuttle' ];
    case 11:
    case 12:
    case 13:
      return [ 'royal' ];
    default:
      return [];
  }
}

export function extractActionSource(actionToken, phaseType = null) {
  return sourceFromParsed(parseLegalActionToken(actionToken), phaseType);
}

export function deriveMoveChoicesForSource(actions = [], source = null, phaseType = null) {
  if (!source) {return [];}
  const choices = actions
    .map((token) => parseLegalActionToken(token))
    .filter((parsed) => sourceMatchesParsed(source, parsed, phaseType))
    .map((parsed) => choiceFromParsed(parsed, phaseType))
    .filter(Boolean)
    .map((type) => {
      const orderIndex = CHOICE_ORDER.indexOf(type);
      return { type, order: orderIndex === -1 ? CHOICE_ORDER.length + 1 : orderIndex };
    });

  return dedupeBy(choices, (choice) => choice.type)
    .sort((a, b) => a.order - b.order)
    .map((choice) => ({ type: choice.type }));
}

export function deriveTargetsForChoice(actions = [], source = null, choice = null, phaseType = null) {
  if (!source || !choice) {return [];}
  return dedupeBy(
    actions
      .map((token) => parseLegalActionToken(token))
      .filter((parsed) => sourceMatchesParsed(source, parsed, phaseType))
      .filter((parsed) => choiceFromParsed(parsed, phaseType) === choice)
      .map((parsed) => targetFromParsed(parsed, phaseType))
      .filter(Boolean)
      .map((target) => ({ ...target, key: targetKey(target) })),
    (target) => target.key,
  );
}

export function findMatchingAction(actions = [], source = null, choice = null, target = null, phaseType = null) {
  if (!source || !choice) {return null;}

  const candidates = actions
    .map((token) => parseLegalActionToken(token))
    .filter((parsed) => sourceMatchesParsed(source, parsed, phaseType))
    .filter((parsed) => choiceFromParsed(parsed, phaseType) === choice);

  if (!target) {
    const targetless = candidates.find((parsed) => !targetFromParsed(parsed, phaseType));
    return targetless?.token ?? candidates[0]?.token ?? null;
  }

  const wantedKey = targetKey(target);
  const match = candidates.find((parsed) => {
    const parsedTarget = targetFromParsed(parsed, phaseType);
    if (!parsedTarget) {return false;}
    if (target.targetType === 'card') {
      return parsedTarget.token === target.token;
    }
    if (target.token) {
      return parsedTarget.token === target.token;
    }
    return targetKey(parsedTarget) === wantedKey;
  });
  return match?.token ?? null;
}

export function deriveCutthroatDialogState({
  phaseType = null,
  legalActions = [],
  selectedSource = null,
  selectedChoice = null,
  targets = [],
} = {}) {
  const parsed = legalActions.map((token) => parseLegalActionToken(token)).filter(Boolean);
  const hasCounterPass = parsed.some((entry) => choiceFromParsed(entry, phaseType) === 'counterPass');
  const counterTwoTokens = dedupeBy(
    parsed
      .filter((entry) => choiceFromParsed(entry, phaseType) === 'counterTwo')
      .map((entry) => normalizeCardToken(entry.args[0]))
      .filter(Boolean)
      .map((token) => ({ token })),
    (entry) => entry.token,
  ).map((entry) => entry.token);

  const resolveFourTokens = parsed
    .filter((entry) => choiceFromParsed(entry, phaseType) === 'resolveFourDiscard')
    .map((entry) => normalizeCardToken(entry.args[1]))
    .filter(Boolean);

  const resolveFiveTokens = parsed
    .filter((entry) => choiceFromParsed(entry, phaseType) === 'resolveFiveDiscard')
    .map((entry) => normalizeCardToken(entry.args[0]))
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
  ) ? rankFromToken(selectedSource.token) : null;

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
    showFourPlayerTargetDialog:
      selectedChoice === 'oneOff' && selectedSourceRank === 4 && playerTargetSeats.length > 0,
  };
}

export function filterVisibleActions(
  actions = [],
  isResolvingSeven = false,
  selectedRevealToken = null,
  phaseType = null,
) {
  if (!isResolvingSeven) {
    return actions;
  }
  return actions.filter((token) => {
    const parsed = parseLegalActionToken(token);
    const source = sourceFromParsed(parsed, phaseType);
    return source?.zone === 'reveal' && source.token === selectedRevealToken;
  });
}

export function groupActions(actions = [], phaseType = null) {
  return actions.reduce(
    (groups, token) => {
      const parsed = parseLegalActionToken(token);
      const choice = choiceFromParsed(parsed, phaseType);
      if (PRIMARY_CHOICES.has(choice)) {
        groups.primary.push(token);
      } else if (COUNTER_CHOICES.has(choice)) {
        groups.counter.push(token);
      } else if (RESOLUTION_CHOICES.has(choice)) {
        groups.resolution.push(token);
      } else {
        groups.other.push(token);
      }
      return groups;
    },
    { primary: [], counter: [], resolution: [], other: [] },
  );
}
