import { nextTick, onBeforeUnmount, onMounted, watch } from 'vue';

export function useCutthroatLifecycle({
  store,
  router,
  t,
  snackbarStore,
  gameId,
  isSpectateRoute,
  isSpectatorMode,
  replayStateIndex,
  legalActions,
  resolveFourHandCards,
  selectedResolveFourTokens,
  resolveFiveDiscardTokens,
  selectedResolveFiveToken,
  syncInteractionState,
  historyLines,
  scrollHistoryLogs,
  smAndDown,
  showHistoryDrawer,
  clearInteractionState,
  actionInFlight,
  actionInFlightKey,
  phaseType,
  isResolvingSeven,
  selectedSource,
  revealedCardEntries,
  isRevealSelectable,
}) {
  let observedStartedDuringSession = false;

  function setBrowserHeightVariable() {
    const viewportHeight = window.innerHeight * 0.01;
    document.documentElement.style.setProperty('--browserHeight', `${viewportHeight}px`);
  }

  async function normalizeFinishedSpectateDeepLinkToReplayStart() {
    const currentRoute = router.currentRoute.value;
    const hasReplayIndex = Object.prototype.hasOwnProperty.call(currentRoute.query, 'gameStateIndex');
    if (!isSpectateRoute.value || hasReplayIndex || store.status !== 2) {return;}
    await router.replace({
      ...currentRoute,
      query: {
        ...currentRoute.query,
        gameStateIndex: 0,
      },
    });
  }

  async function fallbackGameRouteToSpectateReplay() {
    const currentRoute = router.currentRoute.value;
    const hasReplayIndex = Object.prototype.hasOwnProperty.call(currentRoute.query, 'gameStateIndex');
    await store.fetchState(gameId.value, {
      spectateIntent: true,
      gameStateIndex: replayStateIndex.value,
    });
    if (store.status === 1) {
      observedStartedDuringSession = true;
    }
    const query = { ...currentRoute.query };
    if (!hasReplayIndex && store.status === 2) {
      query.gameStateIndex = 0;
    } else if (hasReplayIndex) {
      query.gameStateIndex = replayStateIndex.value;
    }
    await router.replace({
      path: `/cutthroat/spectate/${gameId.value}`,
      query,
    });
  }

  onMounted(async () => {
    setBrowserHeightVariable();
    window.addEventListener('resize', setBrowserHeightVariable);
    await nextTick();
    scrollHistoryLogs();

    try {
      await store.fetchState(gameId.value, {
        spectateIntent: isSpectateRoute.value,
        gameStateIndex: replayStateIndex.value,
      });
      if (store.status === 1 && (!isSpectateRoute.value || replayStateIndex.value < 0)) {
        observedStartedDuringSession = true;
      }
      if (store.status === 0 && !isSpectatorMode.value) {
        router.replace(`/cutthroat/lobby/${gameId.value}`);
        return;
      }
      if (store.status === 0 && isSpectatorMode.value) {
        snackbarStore.alert(t('cutthroat.game.spectateUnavailable'));
        router.push('/');
        return;
      }
      if (isSpectateRoute.value && !isSpectatorMode.value) {
        snackbarStore.alert(t('cutthroat.game.cannotSpectateOwnGame'));
        await router.replace(`/cutthroat/game/${gameId.value}`);
      } else if (!isSpectateRoute.value && isSpectatorMode.value) {
        const currentRoute = router.currentRoute.value;
        const hasReplayIndex = Object.prototype.hasOwnProperty.call(currentRoute.query, 'gameStateIndex');
        const query = { ...currentRoute.query };
        if (!hasReplayIndex && store.status === 2) {
          query.gameStateIndex = 0;
        }
        await router.replace({
          path: `/cutthroat/spectate/${gameId.value}`,
          query,
        });
      }
      await normalizeFinishedSpectateDeepLinkToReplayStart();
    } catch (err) {
      if (isSpectateRoute.value && err?.status === 409) {
        try {
          await store.fetchState(gameId.value, { spectateIntent: false, gameStateIndex: -1 });
          snackbarStore.alert(t('cutthroat.game.cannotSpectateOwnGame'));
          await router.replace(`/cutthroat/game/${gameId.value}`);
          if (store.status === 0) {
            router.replace(`/cutthroat/lobby/${gameId.value}`);
            return;
          }
          if (!store.isArchived) {
            store.connectWs(gameId.value, { spectateIntent: false });
          }
          return;
        } catch (_) {
          snackbarStore.alert(t('cutthroat.game.spectateUnavailable'));
          router.push('/');
          return;
        }
      }
      if (isSpectateRoute.value) {
        snackbarStore.alert(t('cutthroat.game.spectateUnavailable'));
        router.push('/');
        return;
      }
      if (err?.status === 404 || err?.status === 503) {
        try {
          await fallbackGameRouteToSpectateReplay();
        } catch (_) {
          try {
            await store.joinGame(gameId.value);
            await store.fetchState(gameId.value, { spectateIntent: false, gameStateIndex: -1 });
            if (store.status === 0) {
              router.replace(`/cutthroat/lobby/${gameId.value}`);
              return;
            }
          } catch (joinErr) {
            snackbarStore.alert(joinErr?.message ?? t('cutthroat.game.loadFailed'));
            router.push('/');
            return;
          }
        }
      } else {
        try {
          await store.joinGame(gameId.value);
          await store.fetchState(gameId.value, { spectateIntent: false, gameStateIndex: -1 });
          if (store.status === 0) {
            router.replace(`/cutthroat/lobby/${gameId.value}`);
            return;
          }
        } catch (joinErr) {
          snackbarStore.alert(joinErr?.message ?? t('cutthroat.game.loadFailed'));
          router.push('/');
          return;
        }
      }
    }

    if (!store.isArchived) {
      store.connectWs(gameId.value, { spectateIntent: isSpectatorMode.value });
    }
  });

  watch(
    () => replayStateIndex.value,
    async (newIndex, oldIndex) => {
      if (newIndex === oldIndex || !isSpectateRoute.value) {return;}
      try {
        await store.fetchState(gameId.value, {
          spectateIntent: true,
          gameStateIndex: newIndex,
        });
      } catch (_) {
        snackbarStore.alert(t('cutthroat.game.spectateUnavailable'));
        router.push('/');
      }
    },
  );

  watch(
    () => store.status,
    async (status, oldStatus) => {
      if (status === 1 && (!isSpectateRoute.value || replayStateIndex.value < 0)) {
        observedStartedDuringSession = true;
      }
      if (status === 0 && !isSpectatorMode.value) {
        router.replace(`/cutthroat/lobby/${gameId.value}`);
      } else if (status === 0 && isSpectatorMode.value) {
        snackbarStore.alert(t('cutthroat.game.spectateUnavailable'));
        router.push('/');
      }
      if (
        isSpectateRoute.value
        && status === 2
        && oldStatus !== 2
        && observedStartedDuringSession
      ) {
        const currentRoute = router.currentRoute.value;
        if (Number(currentRoute.query.gameStateIndex) !== -1) {
          await router.replace({
            ...currentRoute,
            query: {
              ...currentRoute.query,
              gameStateIndex: -1,
            },
          });
        }
      }
      if (status === 2) {
        clearInteractionState();
      }
    },
  );

  watch(
    () => legalActions.value,
    () => {
      syncInteractionState();
      const allowedFourTokens = new Set(resolveFourHandCards.value.map((entry) => entry.token));
      selectedResolveFourTokens.value = selectedResolveFourTokens.value.filter((token) => allowedFourTokens.has(token));
      if (selectedResolveFiveToken.value && !resolveFiveDiscardTokens.value.includes(selectedResolveFiveToken.value)) {
        selectedResolveFiveToken.value = null;
      }
    },
    { deep: true },
  );

  watch(
    () => historyLines.value,
    () => {
      nextTick(() => {
        scrollHistoryLogs();
      });
    },
    { deep: true },
  );

  watch(
    () => smAndDown.value,
    (isCompact) => {
      if (!isCompact) {
        showHistoryDrawer.value = false;
      }
    },
  );

  watch(
    () => store.lastError,
    (error) => {
      if (!error) {return;}
      snackbarStore.alert(error.message ?? t('cutthroat.game.actionFailed'));
      store.clearLastError();
      actionInFlight.value = false;
      actionInFlightKey.value = '';
      syncInteractionState();
    },
  );

  watch(
    () => gameId.value,
    (id) => {
      if (!id) {
        router.push('/');
      }
    },
  );

  watch(
    () => phaseType.value,
    () => {
      if (isResolvingSeven.value && selectedSource.value?.zone === 'reveal') {
        const revealIndex = revealedCardEntries.value
          .find((entry) => entry.token === selectedSource.value.token)
          ?.index;
        if (!Number.isInteger(revealIndex) || !isRevealSelectable(revealIndex)) {
          clearInteractionState();
        }
      }
    },
  );

  onBeforeUnmount(() => {
    window.removeEventListener('resize', setBrowserHeightVariable);
    store.disconnectWs();
  });
}
