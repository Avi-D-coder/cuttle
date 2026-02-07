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

  return {
    ...useCutthroatInteractions({
      store,
      snackbarStore,
      t: (k) => k,
      legalActions: computed(() => legalActions.value),
      phaseType: computed(() => phaseType.value),
      isActionDisabled: computed(() => false),
      isFinished: computed(() => false),
      isMainPhase: computed(() => phaseType.value === 'Main'),
      isCounteringPhase: computed(() => phaseType.value === 'Countering'),
      isResolvingThree: computed(() => phaseType.value === 'ResolvingThree'),
      isResolvingFour: computed(() => phaseType.value === 'ResolvingFour'),
      isResolvingFive: computed(() => phaseType.value === 'ResolvingFive'),
      myHandCards: computed(() => []),
      myFrozenTokens: computed(() => new Set()),
      revealedCardEntries: computed(() => []),
      isSpectatorMode: computed(() => false),
      isResolvingSeven: computed(() => false),
      localHandActionTokens: computed(() => []),
      cardTokenToDialogCard: () => null,
    }),
    store,
    legalActions,
    phaseType,
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
});
