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
  function setBrowserHeightVariable() {
    const viewportHeight = window.innerHeight * 0.01;
    document.documentElement.style.setProperty('--browserHeight', `${viewportHeight}px`);
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
        router.replace(`/cutthroat/game/${gameId.value}`);
      } else if (!isSpectateRoute.value && isSpectatorMode.value) {
        router.replace(`/cutthroat/spectate/${gameId.value}`);
      }

      const currentRoute = router.currentRoute.value;
      const hasReplayIndex = Object.prototype.hasOwnProperty.call(currentRoute.query, 'gameStateIndex');
      if (isSpectateRoute.value && !hasReplayIndex && store.status === 2) {
        await router.replace({
          ...currentRoute,
          query: {
            ...currentRoute.query,
            gameStateIndex: 0,
          },
        });
      }
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
    (status) => {
      if (status === 0 && !isSpectatorMode.value) {
        router.replace(`/cutthroat/lobby/${gameId.value}`);
      } else if (status === 0 && isSpectatorMode.value) {
        snackbarStore.alert(t('cutthroat.game.spectateUnavailable'));
        router.push('/');
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
