import { computed, ref } from 'vue';
import { parseCardToken } from '@/util/cutthroat-cards';
import {
  deriveCounterDialogContextFromTokenlog,
  deriveCutthroatDialogState,
  deriveMoveChoicesForSource,
  deriveTargetsForChoice,
  findMatchingAction,
} from '@/routes/cutthroat/helpers';

function actionKey(action) {
  return JSON.stringify(action ?? {});
}

function sameSource(a, b) {
  if (!a || !b) {return false;}
  return a.zone === b.zone
    && a.token === b.token
    && a.index === b.index;
}

function makeTargetKey(target) {
  if (!target || !target.targetType) {return '';}
  if (target.targetType === 'player') {
    return `player:${target.seat}`;
  }
  if (target.token !== undefined && target.token !== null) {
    return `${target.targetType}:${target.token}`;
  }
  return target.targetType;
}

export function useCutthroatInteractions({
  store,
  snackbarStore,
  t,
  legalActions,
  phaseType,
  isActionDisabled,
  isFinished,
  isMainPhase,
  isCounteringPhase,
  isResolvingThree,
  isResolvingFour,
  isResolvingFive,
  myHandCards,
  myFrozenTokens,
  revealedCardEntries,
  isSpectatorMode,
  localHandActionTokens,
  cardTokenToDialogCard,
}) {
  const actionInFlight = ref(false);
  const actionInFlightKey = ref('');
  const selectedSource = ref(null);
  const selectedChoice = ref(null);
  const selectedResolveFourTokens = ref([]);
  const selectedResolveFiveToken = ref(null);

  const selectedSourceChoices = computed(() => {
    return deriveMoveChoicesForSource(legalActions.value, selectedSource.value);
  });

  const selectedChoiceTargets = computed(() => {
    if (!selectedSource.value || !selectedChoice.value) {return [];}
    return deriveTargetsForChoice(legalActions.value, selectedSource.value, selectedChoice.value);
  });

  const targetKeySet = computed(() => {
    return new Set(selectedChoiceTargets.value.map((target) => target.key));
  });

  const dialogState = computed(() => {
    return deriveCutthroatDialogState({
      phaseType: phaseType.value,
      legalActions: legalActions.value,
      selectedSource: selectedSource.value,
      selectedChoice: selectedChoice.value,
      targets: selectedChoiceTargets.value,
    });
  });

  const isTargeting = computed(() => {
    return !!selectedSource.value && !!selectedChoice.value && selectedChoiceTargets.value.length > 0;
  });

  const playerTargetChoices = computed(() => {
    return dialogState.value.playerTargetSeats.map((seat) => ({
      targetType: 'player',
      seat,
      key: `player:${seat}`,
    }));
  });

  const showFourPlayerTargetDialog = computed(() => {
    return isTargeting.value && dialogState.value.showFourPlayerTargetDialog;
  });

  const showMoveChoiceOverlay = computed(() => {
    if (!selectedSource.value || selectedChoice.value) {return false;}
    if (selectedSource.value.zone === 'hand') {return true;}
    return selectedSourceChoices.value.length > 0;
  });

  const hasCounterPassAction = computed(() => {
    return dialogState.value.hasCounterPass;
  });

  const isCounterTurn = computed(() => {
    if (!isCounteringPhase.value) {return false;}
    return hasCounterPassAction.value;
  });

  const isResolvingThreeTurn = computed(() => {
    if (!isResolvingThree.value) {return false;}
    return legalActions.value.some((action) => action?.type === 'ResolveThreePick');
  });

  const counterTwoOptions = computed(() => {
    return dialogState.value.counterTwoTokens;
  });

  const counterContext = computed(() => {
    if (!isCounteringPhase.value) {return null;}
    return deriveCounterDialogContextFromTokenlog(store.tokenlog);
  });

  const counterDialogOneOff = computed(() => {
    return cardTokenToDialogCard(counterContext.value?.oneOffCardToken ?? null);
  });

  const counterDialogTarget = computed(() => {
    const target = counterContext.value?.oneOffTarget ?? null;
    if (!target) {return null;}
    switch (target.type) {
      case 'Point':
      case 'Royal':
      case 'Jack':
      case 'Joker':
        return cardTokenToDialogCard(target.token);
      default:
        return null;
    }
  });

  const counterDialogTwosPlayed = computed(() => {
    return (counterContext.value?.twosPlayed ?? [])
      .map((token) => {
        const card = cardTokenToDialogCard(token);
        if (!card) {return null;}
        return {
          ...card,
          id: token,
        };
      })
      .filter(Boolean);
  });

  const counterDialogTwosInHand = computed(() => {
    return counterTwoOptions.value
      .map((token) => cardTokenToDialogCard(token))
      .filter(Boolean);
  });

  const showCounterDialog = computed(() => {
    return dialogState.value.showCounterDialog;
  });

  const showCannotCounterDialog = computed(() => {
    return dialogState.value.showCannotCounterDialog;
  });

  const counterDialogInvariantError = computed(() => {
    if (!isCounteringPhase.value) {return false;}
    if (!showCounterDialog.value && !showCannotCounterDialog.value) {return false;}
    return !counterContext.value || !counterDialogOneOff.value;
  });

  const canUseDeck = computed(() => {
    if (isActionDisabled.value || isFinished.value || !isMainPhase.value) {return false;}
    const draw = findMatchingAction(legalActions.value, { zone: 'deck' }, 'draw');
    const pass = findMatchingAction(legalActions.value, { zone: 'deck' }, 'pass');
    return !!(draw || pass);
  });

  const selectedSourceCard = computed(() => {
    if (!selectedSource.value) {return null;}
    if (selectedSource.value.zone === 'hand') {
      const found = myHandCards.value.find((card) => card.token === selectedSource.value.token);
      return found?.card ?? parseCardToken(selectedSource.value.token);
    }
    if (selectedSource.value.zone === 'reveal') {
      const found = revealedCardEntries.value.find((entry) => entry.index === selectedSource.value.index);
      return found?.card ?? null;
    }
    if (selectedSource.value.zone === 'scrap') {
      return parseCardToken(selectedSource.value.token);
    }
    return null;
  });

  const selectedSourceIsFrozen = computed(() => {
    if (!selectedSource.value || selectedSource.value.zone !== 'hand') {return false;}
    return isFrozenToken(selectedSource.value.token);
  });

  const resolveFiveActions = computed(() => {
    return legalActions.value.filter((action) => action?.type === 'ResolveFiveDiscard');
  });

  const resolveFourDiscardTokens = computed(() => {
    return dialogState.value.resolveFourTokens;
  });

  const resolveFiveDiscardTokens = computed(() => {
    return dialogState.value.resolveFiveTokens;
  });

  const showResolveFourDialog = computed(() => {
    return dialogState.value.showResolveFourDialog;
  });

  const showResolveFiveDialog = computed(() => {
    return dialogState.value.showResolveFiveDialog;
  });

  const resolveFourHandCards = computed(() => {
    return resolveFourDiscardTokens.value.map((token, index) => ({
      token,
      key: `resolve-four-${token}-${index}`,
      card: parseCardToken(token),
    }));
  });

  const resolveFiveHandCards = computed(() => {
    return resolveFiveDiscardTokens.value.map((token, index) => ({
      token,
      key: `resolve-five-${token}-${index}`,
      card: parseCardToken(token),
    }));
  });

  const canSubmitResolveFour = computed(() => {
    if (resolveFourHandCards.value.length === 0) {return false;}
    const maxSelectable = Math.min(2, resolveFourHandCards.value.length);
    return selectedResolveFourTokens.value.length === maxSelectable;
  });

  const canSubmitResolveFive = computed(() => {
    if (resolveFiveHandCards.value.length === 0) {return true;}
    return !!selectedResolveFiveToken.value;
  });

  function isActionLoading(action) {
    if (!actionInFlight.value) {return false;}
    return actionInFlightKey.value === actionKey(action);
  }

  function clearInteractionState() {
    selectedSource.value = null;
    selectedChoice.value = null;
  }

  function cancelTargeting() {
    selectedChoice.value = null;
  }

  function syncInteractionState() {
    if (!selectedSource.value) {return;}

    const choices = deriveMoveChoicesForSource(legalActions.value, selectedSource.value);
    if (choices.length === 0) {
      clearInteractionState();
      return;
    }

    if (!selectedChoice.value) {return;}

    const matchingChoice = choices.some((choice) => choice.type === selectedChoice.value);
    if (!matchingChoice) {
      selectedChoice.value = null;
      return;
    }

    const targets = deriveTargetsForChoice(legalActions.value, selectedSource.value, selectedChoice.value);
    if (targets.length === 0) {
      selectedChoice.value = null;
    }
  }

  function isFrozenToken(token) {
    if (!token) {return false;}
    return myFrozenTokens.value.has(token);
  }

  function isHandSourceSelectable(handCard) {
    if (!handCard?.isKnown || !handCard?.token || isActionDisabled.value || isFinished.value) {return false;}
    return true;
  }

  function isHandSourceSelected(handCard) {
    if (!handCard?.token || !selectedSource.value) {return false;}
    return selectedSource.value.zone === 'hand' && selectedSource.value.token === handCard.token;
  }

  function isRevealSelectable(index) {
    const source = {
      zone: 'reveal',
      index,
    };
    return deriveMoveChoicesForSource(legalActions.value, source).length > 0;
  }

  function isRevealSelected(index) {
    if (!selectedSource.value) {return false;}
    return selectedSource.value.zone === 'reveal' && selectedSource.value.index === index;
  }

  function hasTarget(target) {
    return targetKeySet.value.has(makeTargetKey(target));
  }

  function isPointTarget(token) {
    return isTargeting.value && hasTarget({
      targetType: 'point',
      token,
    });
  }

  function isRoyalTarget(token) {
    return isTargeting.value && hasTarget({
      targetType: 'royal',
      token,
    });
  }

  function isJackTarget(token) {
    return isTargeting.value && hasTarget({
      targetType: 'jack',
      token,
    });
  }

  function isJokerTarget(token) {
    return isTargeting.value && hasTarget({
      targetType: 'joker',
      token,
    });
  }

  function isPlayerTarget(seat) {
    return isTargeting.value && hasTarget({
      targetType: 'player',
      seat,
    });
  }

  async function sendResolvedAction(action) {
    if (!action || isActionDisabled.value) {return false;}

    actionInFlight.value = true;
    actionInFlightKey.value = actionKey(action);
    let succeeded = false;

    try {
      await store.sendAction(action);
      succeeded = true;
    } catch (err) {
      if (!store.lastError) {
        snackbarStore.alert(err?.message ?? t('cutthroat.game.actionFailed'));
      }
    } finally {
      actionInFlight.value = false;
      actionInFlightKey.value = '';
    }

    if (succeeded) {
      clearInteractionState();
    } else {
      syncInteractionState();
    }

    return succeeded;
  }

  async function executeSourceChoice(source, choiceType, target = null) {
    const action = findMatchingAction(legalActions.value, source, choiceType, target);
    if (!action) {return;}
    await sendResolvedAction(action);
  }

  function chooseMove(choiceType) {
    if (!selectedSource.value || isActionDisabled.value) {return;}

    const targets = deriveTargetsForChoice(legalActions.value, selectedSource.value, choiceType);
    if (targets.length === 0) {
      executeSourceChoice(selectedSource.value, choiceType);
      return;
    }

    selectedChoice.value = choiceType;
  }

  function resolveTargetSelection(target) {
    if (!selectedSource.value || !selectedChoice.value || !isTargeting.value) {return;}
    if (!hasTarget(target)) {return;}
    executeSourceChoice(selectedSource.value, selectedChoice.value, target);
  }

  async function handleDeckClick() {
    if (!canUseDeck.value) {return;}
    const draw = findMatchingAction(legalActions.value, { zone: 'deck' }, 'draw');
    const pass = findMatchingAction(legalActions.value, { zone: 'deck' }, 'pass');
    await sendResolvedAction(draw ?? pass);
  }

  async function handleCounterPass() {
    if (isSpectatorMode.value) {return;}
    const action = findMatchingAction(legalActions.value, { zone: 'counter', token: 'pass' }, 'counterPass');
    await sendResolvedAction(action);
  }

  async function handleCounterTwo(twoToken) {
    if (isSpectatorMode.value) {return;}
    if (!twoToken) {return;}
    const action = findMatchingAction(
      legalActions.value,
      { zone: 'hand', token: twoToken },
      'counterTwo',
    );
    await sendResolvedAction(action);
  }

  async function handleCounterTwoFromDialog(twoId) {
    if (!twoId) {return;}
    await handleCounterTwo(twoId);
  }

  function toggleResolveFourCard(token) {
    if (isSpectatorMode.value) {return;}
    if (!token) {return;}
    const selected = selectedResolveFourTokens.value;
    if (selected.includes(token)) {
      selectedResolveFourTokens.value = selected.filter((entry) => entry !== token);
      return;
    }
    const maxSelectable = Math.min(2, resolveFourHandCards.value.length);
    const next = [ ...selected, token ];
    if (next.length > maxSelectable) {
      next.shift();
    }
    selectedResolveFourTokens.value = next;
  }

  async function submitResolveFourDiscard() {
    if (isSpectatorMode.value) {return;}
    if (!canSubmitResolveFour.value) {return;}
    const tokens = [ ...selectedResolveFourTokens.value ];
    for (const token of tokens) {
      const action = findMatchingAction(
        legalActions.value,
        { zone: 'hand', token },
        'resolveFourDiscard',
      );
      if (!action) {continue;}
      await sendResolvedAction(action);
    }
    selectedResolveFourTokens.value = [];
  }

  async function submitResolveFiveDiscard() {
    if (isSpectatorMode.value) {return;}
    if (!canSubmitResolveFive.value) {return;}
    let action = null;
    if (selectedResolveFiveToken.value) {
      action = findMatchingAction(
        legalActions.value,
        { zone: 'hand', token: selectedResolveFiveToken.value },
        'resolveFiveDiscard',
      );
    } else {
      action = resolveFiveActions.value[0] ?? null;
    }
    await sendResolvedAction(action);
    selectedResolveFiveToken.value = null;
  }

  async function handleHandCardClick(handCard) {
    if (!isHandSourceSelectable(handCard)) {return;}

    const source = {
      zone: 'hand',
      token: handCard.token,
    };
    const choices = deriveMoveChoicesForSource(legalActions.value, source);

    if (
      isResolvingFour.value
      || isResolvingFive.value
      || (isCounterTurn.value && choices.length === 1 && choices[0].type === 'counterTwo')
    ) {
      await executeSourceChoice(source, choices[0].type);
      return;
    }

    if (sameSource(selectedSource.value, source) && !selectedChoice.value) {
      clearInteractionState();
      return;
    }

    if (isResolvingFour.value || isResolvingFive.value) {return;}

    selectedSource.value = source;
    selectedChoice.value = null;
  }

  function handleRevealClick(index) {
    if (isActionDisabled.value) {return;}

    const source = {
      zone: 'reveal',
      index,
    };

    if (deriveMoveChoicesForSource(legalActions.value, source).length === 0) {return;}

    if (sameSource(selectedSource.value, source) && !selectedChoice.value) {
      clearInteractionState();
      return;
    }

    selectedSource.value = source;
    selectedChoice.value = null;
  }

  async function handleScrapCardClick(token) {
    if (!isResolvingThreeTurn.value || isActionDisabled.value) {return;}

    await executeSourceChoice(
      {
        zone: 'scrap',
        token,
      },
      'resolveThreePick',
    );
  }

  function handleRequestScrapStraighten() {
    try {
      store.sendScrapStraighten();
    } catch (err) {
      snackbarStore.alert(err?.message ?? t('cutthroat.game.actionFailed'));
    }
  }

  function handlePointTargetClick(token) {
    resolveTargetSelection({
      targetType: 'point',
      token,
    });
  }

  function handleRoyalTargetClick(token) {
    resolveTargetSelection({
      targetType: 'royal',
      token,
    });
  }

  function handleJackTargetClick(token) {
    resolveTargetSelection({
      targetType: 'jack',
      token,
    });
  }

  function handleJokerTargetClick(token) {
    resolveTargetSelection({
      targetType: 'joker',
      token,
    });
  }

  function handlePlayerTargetClick(seat) {
    resolveTargetSelection({
      targetType: 'player',
      seat,
    });
  }

  return {
    actionInFlight,
    actionInFlightKey,
    selectedSource,
    selectedChoice,
    selectedResolveFourTokens,
    selectedResolveFiveToken,
    selectedSourceChoices,
    selectedChoiceTargets,
    dialogState,
    isTargeting,
    playerTargetChoices,
    showFourPlayerTargetDialog,
    showMoveChoiceOverlay,
    hasCounterPassAction,
    isCounterTurn,
    isResolvingThreeTurn,
    localHandActionTokens,
    counterTwoOptions,
    counterContext,
    counterDialogOneOff,
    counterDialogTarget,
    counterDialogTwosPlayed,
    counterDialogTwosInHand,
    showCounterDialog,
    showCannotCounterDialog,
    counterDialogInvariantError,
    canUseDeck,
    selectedSourceCard,
    selectedSourceIsFrozen,
    resolveFiveActions,
    resolveFourDiscardTokens,
    resolveFiveDiscardTokens,
    showResolveFourDialog,
    showResolveFiveDialog,
    resolveFourHandCards,
    resolveFiveHandCards,
    canSubmitResolveFour,
    canSubmitResolveFive,
    isActionLoading,
    clearInteractionState,
    cancelTargeting,
    syncInteractionState,
    isFrozenToken,
    isHandSourceSelectable,
    isHandSourceSelected,
    isRevealSelectable,
    isRevealSelected,
    isPointTarget,
    isRoyalTarget,
    isJackTarget,
    isJokerTarget,
    isPlayerTarget,
    sendResolvedAction,
    chooseMove,
    handleDeckClick,
    handleCounterPass,
    handleCounterTwo,
    handleCounterTwoFromDialog,
    toggleResolveFourCard,
    submitResolveFourDiscard,
    submitResolveFiveDiscard,
    handleHandCardClick,
    handleRevealClick,
    handleScrapCardClick,
    handleRequestScrapStraighten,
    handlePointTargetClick,
    handleRoyalTargetClick,
    handleJackTargetClick,
    handleJokerTargetClick,
    handlePlayerTargetClick,
  };
}
