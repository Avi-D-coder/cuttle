import { describe, expect, it, vi } from 'vitest';

async function loadLifecycleComposable() {
  const watches = [];
  let onMountedCb = null;
  let onBeforeUnmountCb = null;

  vi.doMock('vue', () => ({
    nextTick: vi.fn((cb) => (cb ? Promise.resolve(cb()) : Promise.resolve())),
    onMounted: vi.fn((cb) => {
      onMountedCb = cb;
    }),
    onBeforeUnmount: vi.fn((cb) => {
      onBeforeUnmountCb = cb;
    }),
    watch: vi.fn((source, cb) => {
      watches.push({ source, cb });
    }),
  }));

  const mod = await import('@/routes/cutthroat/composables/useCutthroatLifecycle');
  return {
    useCutthroatLifecycle: mod.useCutthroatLifecycle,
    watches,
    getOnMountedCb: () => onMountedCb,
    getOnBeforeUnmountCb: () => onBeforeUnmountCb,
  };
}

function buildLifecycleArgs(overrides = {}) {
  return {
    store: {
      status: 1,
      lastError: null,
      clearLastError: vi.fn(),
      disconnectWs: vi.fn(),
      fetchState: vi.fn(async () => {}),
      joinGame: vi.fn(async () => {}),
      connectWs: vi.fn(),
      ...overrides.store,
    },
    router: {
      push: vi.fn(),
      replace: vi.fn(async () => {}),
      ...overrides.router,
    },
    t: overrides.t ?? ((k) => k),
    snackbarStore: {
      alert: vi.fn(),
      ...overrides.snackbarStore,
    },
    gameId: overrides.gameId ?? { value: 1 },
    isSpectateRoute: overrides.isSpectateRoute ?? { value: false },
    isSpectatorMode: overrides.isSpectatorMode ?? { value: false },
    replayStateIndex: overrides.replayStateIndex ?? { value: -1 },
    legalActions: overrides.legalActions ?? { value: [] },
    resolveFourHandCards: overrides.resolveFourHandCards ?? { value: [] },
    selectedResolveFourTokens: overrides.selectedResolveFourTokens ?? { value: [] },
    resolveFiveDiscardTokens: overrides.resolveFiveDiscardTokens ?? { value: [] },
    selectedResolveFiveToken: overrides.selectedResolveFiveToken ?? { value: null },
    syncInteractionState: overrides.syncInteractionState ?? vi.fn(),
    historyLines: overrides.historyLines ?? { value: [] },
    scrollHistoryLogs: overrides.scrollHistoryLogs ?? vi.fn(),
    smAndDown: overrides.smAndDown ?? { value: false },
    showHistoryDrawer: overrides.showHistoryDrawer ?? { value: false },
    clearInteractionState: overrides.clearInteractionState ?? vi.fn(),
    actionInFlight: overrides.actionInFlight ?? { value: false },
    actionInFlightKey: overrides.actionInFlightKey ?? { value: '' },
    phaseType: overrides.phaseType ?? { value: 'Main' },
    isResolvingSeven: overrides.isResolvingSeven ?? { value: false },
    selectedSource: overrides.selectedSource ?? { value: null },
    isRevealSelectable: overrides.isRevealSelectable ?? vi.fn(() => true),
  };
}

describe('useCutthroatLifecycle', () => {
  it('clears reveal selection when phase changes and reveal is no longer selectable', async () => {
    vi.resetModules();
    const { useCutthroatLifecycle, watches } = await loadLifecycleComposable();

    const clearInteractionState = vi.fn();

    const phaseType = { value: 'Main' };
    const isResolvingSeven = { value: true };
    const selectedSource = { value: { zone: 'reveal', index: 0 } };

    useCutthroatLifecycle(buildLifecycleArgs({
      clearInteractionState,
      phaseType,
      isResolvingSeven,
      selectedSource,
      isRevealSelectable: vi.fn(() => false),
    }));

    const phaseWatch = watches.find((entry) => entry.source() === 'Main');
    phaseWatch.cb();

    expect(clearInteractionState).toHaveBeenCalledTimes(1);
  });

  it('resets action-in-flight state and syncs when store.lastError changes', async () => {
    vi.resetModules();
    const { useCutthroatLifecycle, watches } = await loadLifecycleComposable();

    const actionInFlight = { value: true };
    const actionInFlightKey = { value: 'abc' };
    const syncInteractionState = vi.fn();
    const snackbarStore = { alert: vi.fn() };
    const store = {
      status: 1,
      lastError: null,
      clearLastError: vi.fn(),
      disconnectWs: vi.fn(),
      fetchState: vi.fn(),
      joinGame: vi.fn(),
      connectWs: vi.fn(),
    };

    const args = buildLifecycleArgs({
      store,
      snackbarStore,
      syncInteractionState,
      actionInFlight,
      actionInFlightKey,
    });
    useCutthroatLifecycle(args);

    args.store.lastError = { message: 'illegal action' };
    const errorWatch = watches.find((entry) => entry.source() === args.store.lastError);
    errorWatch.cb(args.store.lastError);

    expect(snackbarStore.alert).toHaveBeenCalledWith('illegal action');
    expect(store.clearLastError).toHaveBeenCalledTimes(1);
    expect(actionInFlight.value).toBe(false);
    expect(actionInFlightKey.value).toBe('');
    expect(syncInteractionState).toHaveBeenCalledTimes(1);
  });

  it('non-spectate 409 falls back to join flow instead of spectate-unavailable alert', async () => {
    vi.resetModules();
    vi.stubGlobal('window', {
      innerHeight: 1000,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    });
    vi.stubGlobal('document', {
      documentElement: {
        style: {
          setProperty: vi.fn(),
        },
      },
    });
    const { useCutthroatLifecycle, getOnMountedCb } = await loadLifecycleComposable();

    const fetchState = vi.fn()
      .mockRejectedValueOnce({ status: 409 })
      .mockResolvedValueOnce({});
    const joinGame = vi.fn(async () => {});
    const connectWs = vi.fn();
    const snackbarStore = { alert: vi.fn() };

    useCutthroatLifecycle(buildLifecycleArgs({
      store: {
        status: 1,
        lastError: null,
        clearLastError: vi.fn(),
        disconnectWs: vi.fn(),
        fetchState,
        joinGame,
        connectWs,
      },
      snackbarStore,
      isSpectateRoute: { value: false },
      isSpectatorMode: { value: false },
    }));

    await getOnMountedCb()();

    expect(joinGame).toHaveBeenCalledWith(1);
    expect(snackbarStore.alert).not.toHaveBeenCalledWith('cutthroat.game.spectateUnavailable');
    vi.unstubAllGlobals();
  });

  it('spectate 409 shows spectate-unavailable when non-spectate fetch also fails', async () => {
    vi.resetModules();
    vi.stubGlobal('window', {
      innerHeight: 1000,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    });
    vi.stubGlobal('document', {
      documentElement: {
        style: {
          setProperty: vi.fn(),
        },
      },
    });
    const { useCutthroatLifecycle, getOnMountedCb } = await loadLifecycleComposable();
    const router = {
      push: vi.fn(),
      replace: vi.fn(async () => {}),
    };
    const snackbarStore = { alert: vi.fn() };

    useCutthroatLifecycle(buildLifecycleArgs({
      store: {
        status: 1,
        lastError: null,
        clearLastError: vi.fn(),
        disconnectWs: vi.fn(),
        fetchState: vi.fn().mockRejectedValue({ status: 409 }),
        joinGame: vi.fn(),
        connectWs: vi.fn(),
      },
      router,
      snackbarStore,
      isSpectateRoute: { value: true },
      isSpectatorMode: { value: true },
    }));

    await getOnMountedCb()();

    expect(snackbarStore.alert).toHaveBeenCalledWith('cutthroat.game.spectateUnavailable');
    expect(router.push).toHaveBeenCalledWith('/');
    vi.unstubAllGlobals();
  });
});
