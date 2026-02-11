import { computed, ref } from 'vue';
import { describe, expect, it, vi } from 'vitest';
import { useCutthroatInteractions } from '@/routes/cutthroat/composables/useCutthroatInteractions';

function buildInteractions(overrides = {}) {
  const store = {
    tokenlog: '',
    lastError: null,
    sendAction: vi.fn(async () => {}),
    sendScrapStraighten: vi.fn(() => {}),
    ...overrides.store,
  };
  const snackbarStore = {
    alert: vi.fn(),
  };

  const legalActions = ref(overrides.legalActions ?? []);
  const phaseType = ref(overrides.phaseType ?? 'Main');
  const isActionDisabled = ref(overrides.isActionDisabled ?? false);
  const isFinished = ref(overrides.isFinished ?? false);
  const myHandCards = ref(overrides.myHandCards ?? []);
  const isSpectatorMode = ref(overrides.isSpectatorMode ?? false);
  const deckCount = ref(overrides.deckCount ?? 10);

  return {
    ...useCutthroatInteractions({
      store,
      snackbarStore,
      t: (k) => k,
      legalActions: computed(() => legalActions.value),
      deckCount: computed(() => deckCount.value),
      phaseType: computed(() => phaseType.value),
      isActionDisabled: computed(() => isActionDisabled.value),
      isFinished: computed(() => isFinished.value),
      isMainPhase: computed(() => phaseType.value === 'Main'),
      isCounteringPhase: computed(() => phaseType.value === 'Countering'),
      isResolvingThree: computed(() => phaseType.value === 'ResolvingThree'),
      isResolvingFour: computed(() => phaseType.value === 'ResolvingFour'),
      isResolvingFive: computed(() => phaseType.value === 'ResolvingFive'),
      myHandCards: computed(() => myHandCards.value),
      myFrozenTokens: computed(() => new Set()),
      revealedCardEntries: computed(() => []),
      isSpectatorMode: computed(() => isSpectatorMode.value),
      isResolvingSeven: computed(() => false),
      replayStateIndex: computed(() => -1),
      localHandActionTokens: computed(() => []),
      cardTokenToDialogCard: () => null,
    }),
    store,
    legalActions,
    phaseType,
    isActionDisabled,
    isFinished,
    myHandCards,
    isSpectatorMode,
    deckCount,
    snackbarStore,
  };
}

describe('useCutthroatInteractions', () => {
  it('submits resolve-four discards sequentially', async () => {
    const legalActions = [
      'P1 resolve discard 7C',
      'P1 resolve discard 8D',
    ];
    const sent = [];
    const { store, selectedResolveFourTokens, submitResolveFourDiscard } = buildInteractions({
      phaseType: 'ResolvingFour',
      legalActions,
      store: {
        sendAction: vi.fn(async (actionToken) => {
          sent.push(actionToken.split(' ').at(-1));
        }),
      },
    });

    selectedResolveFourTokens.value = [ '7C', '8D' ];
    await submitResolveFourDiscard();

    expect(sent).toEqual([ '7C', '8D' ]);
    expect(store.sendAction).toHaveBeenCalledTimes(2);
    expect(selectedResolveFourTokens.value).toEqual([]);
  });

  it('allows spectators to select a hand card even when interactions are disabled', async () => {
    const handCard = {
      isKnown: true,
      token: 'J0',
      key: 'J0',
      card: { kind: 'joker', id: 0 },
    };
    const { selectedSource, handleHandCardClick, store } = buildInteractions({
      isActionDisabled: true,
      isSpectatorMode: true,
      myHandCards: [ handCard ],
    });

    await handleHandCardClick(handCard);

    expect(selectedSource.value).toEqual({
      zone: 'hand',
      token: 'J0',
    });
    expect(store.sendAction).not.toHaveBeenCalled();
  });

  it('keeps non-spectators from selecting hand cards when interactions are disabled', async () => {
    const handCard = {
      isKnown: true,
      token: '7C',
      key: '7C',
      card: { kind: 'standard', rank: 7, suit: 0 },
    };
    const { selectedSource, handleHandCardClick } = buildInteractions({
      isActionDisabled: true,
      isSpectatorMode: false,
      myHandCards: [ handCard ],
    });

    await handleHandCardClick(handCard);

    expect(selectedSource.value).toBeNull();
  });

  it('does not send gameplay actions when a spectator clicks a resolve-four card', async () => {
    const handCard = {
      isKnown: true,
      token: '7C',
      key: '7C',
      card: { kind: 'standard', rank: 7, suit: 0 },
    };
    const { store, handleHandCardClick } = buildInteractions({
      phaseType: 'ResolvingFour',
      isActionDisabled: true,
      isSpectatorMode: true,
      myHandCards: [ handCard ],
      legalActions: [
        'P1 resolve discard 7C',
      ],
    });

    await handleHandCardClick(handCard);

    expect(store.sendAction).not.toHaveBeenCalled();
  });

  it('submits typed target clicks for one-off card targets', async () => {
    const handCard = {
      isKnown: true,
      token: '9C',
      key: '9C',
      card: { kind: 'standard', rank: 9, suit: 0 },
    };
    const { store, handleHandCardClick, chooseMove, handleRoyalTargetClick } = buildInteractions({
      legalActions: [
        'P1 oneOff 9C QH',
      ],
      myHandCards: [ handCard ],
      store: {
        sendAction: vi.fn(async () => {}),
      },
    });

    await handleHandCardClick(handCard);
    chooseMove('oneOff');
    handleRoyalTargetClick('QH');

    expect(store.sendAction).toHaveBeenCalledWith('P1 oneOff 9C QH');
  });

  it('supports requesting stalemate from legal propose token', async () => {
    const { store, requestStalemate, waitingForOpponentStalemate } = buildInteractions({
      legalActions: [ 'P1 stalemate-propose' ],
      store: {
        sendAction: vi.fn(async () => {}),
      },
    });

    const succeeded = await requestStalemate();

    expect(succeeded).toBe(true);
    expect(store.sendAction).toHaveBeenCalledWith('P1 stalemate-propose');
    expect(waitingForOpponentStalemate.value).toBe(true);
  });

  it('supports accepting and rejecting opponent stalemate request', async () => {
    const { store, acceptStalemate, rejectStalemate, consideringOpponentStalemateRequest } = buildInteractions({
      legalActions: [ 'P1 stalemate-accept', 'P1 stalemate-reject' ],
      store: {
        sendAction: vi.fn(async () => {}),
      },
    });

    expect(consideringOpponentStalemateRequest.value).toBe(true);

    const accepted = await acceptStalemate();
    const rejected = await rejectStalemate();

    expect(accepted).toBe(true);
    expect(rejected).toBe(true);
    expect(store.sendAction).toHaveBeenNthCalledWith(1, 'P1 stalemate-accept');
    expect(store.sendAction).toHaveBeenNthCalledWith(2, 'P1 stalemate-reject');
  });

  it('shows hand-limit snackbar when deck click cannot draw despite cards in deck', async () => {
    const { handleDeckClick, store, snackbarStore } = buildInteractions({
      phaseType: 'Main',
      deckCount: 5,
      legalActions: [ 'P1 points 7C' ],
    });

    await handleDeckClick();

    expect(store.sendAction).not.toHaveBeenCalled();
    expect(snackbarStore.alert).toHaveBeenCalledWith('game.snackbar.draw.handLimit');
  });
});
