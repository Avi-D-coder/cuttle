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
