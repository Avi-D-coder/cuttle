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

  return {
    ...useCutthroatInteractions({
      store,
      snackbarStore,
      t: (k) => k,
      legalActions: computed(() => legalActions.value),
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
  };
}

describe('useCutthroatInteractions', () => {
  it('submits resolve-four discards sequentially', async () => {
    const legalActions = [
      { type: 'ResolveFourDiscard', data: { card: '7C' } },
      { type: 'ResolveFourDiscard', data: { card: '8D' } },
    ];
    const sent = [];
    const { store, selectedResolveFourTokens, submitResolveFourDiscard } = buildInteractions({
      phaseType: 'ResolvingFour',
      legalActions,
      store: {
        sendAction: vi.fn(async (action) => {
          sent.push(action.data.card);
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
        { type: 'ResolveFourDiscard', data: { card: '7C' } },
      ],
    });

    await handleHandCardClick(handCard);

    expect(store.sendAction).not.toHaveBeenCalled();
  });
});
