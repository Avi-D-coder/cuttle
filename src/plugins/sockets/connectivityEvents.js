import { getActivePinia } from 'pinia';
import { useAuthStore } from '@/stores/auth';
import { useGameStore } from '@/stores/game';
import { useGameHistoryStore } from '@/stores/gameHistory';
import { useGameListStore } from '@/stores/gameList';
import { useSnackbarStore } from '@/stores/snackbar';
import i18n from '@/plugins/i18n';
import router from '@/router.js';

// Keep route names local to avoid loading named exports from router during module init.
const ROUTE_NAME_GAME = 'Game';
const ROUTE_NAME_VS_AI = 'VsAi';
const ROUTE_NAME_SPECTATE = 'Spectate';
const ROUTE_NAME_HOME = 'Home';
const ROUTE_NAME_LOBBY = 'Lobby';

/*
  Routes in ROUTES_REQUIRING_SOCKET depend on live websocket updates.
  On those routes we:
  - auto-reconnect on disconnect/connect_error,
  - skip auto-reconnect when socket.io reports `io client disconnect`,
  - show a reconnect error only if still disconnected after 10s.
*/
const RECONNECT_ERROR_DELAY_MS = 10000;
const ROUTES_REQUIRING_SOCKET = [
  ROUTE_NAME_GAME,
  ROUTE_NAME_VS_AI,
  ROUTE_NAME_LOBBY,
  ROUTE_NAME_SPECTATE,
  ROUTE_NAME_HOME,
];

let reconnectErrorTimeout = null;
let reconnectErrorMessage = null;

function routeRequiresSocket(routeName = router?.currentRoute?.value?.name) {
  return ROUTES_REQUIRING_SOCKET.includes(routeName);
}

function hasActivePinia() {
  return !!getActivePinia();
}

// Cancel the delayed reconnect timer and clear only the reconnect snackbar we created.
function clearReconnectError() {
  if (reconnectErrorTimeout) {
    clearTimeout(reconnectErrorTimeout);
    reconnectErrorTimeout = null;
  }
  if (!reconnectErrorMessage) {
    return;
  }
  if (!hasActivePinia()) {
    reconnectErrorMessage = null;
    return;
  }

  const snackbarStore = useSnackbarStore();
  if (snackbarStore.getShowSnackbar && snackbarStore.getSnackMessage === reconnectErrorMessage) {
    snackbarStore.clear();
  }
  reconnectErrorMessage = null;
}

// Show reconnect failure only after a sustained outage to avoid flashing on brief blips.
function scheduleReconnectError(io) {
  if (!hasActivePinia() || reconnectErrorTimeout || !routeRequiresSocket() || io.socket.isConnected()) {
    return;
  }

  reconnectErrorTimeout = setTimeout(() => {
    reconnectErrorTimeout = null;
    if (!hasActivePinia() || !routeRequiresSocket() || io.socket.isConnected()) {
      return;
    }
    const snackbarStore = useSnackbarStore();
    reconnectErrorMessage = i18n.global.t('global.socket.reconnectFailed');
    snackbarStore.alert(reconnectErrorMessage, 'error', 0);
  }, RECONNECT_ERROR_DELAY_MS);
}

// Trigger reconnect if the client is disconnected and not already connecting.
function requestReconnect(io) {
  if (io.socket.isConnected() || io.socket.isConnecting()) {
    return;
  }
  try {
    io.socket.reconnect();
  } catch (err) {
    console.warn('Socket reconnect request failed:', err);
  }
}

// Common disconnected-state handling used by disconnect and connect_error.
function handleDisconnectedState(io) {
  if (!routeRequiresSocket()) {
    return;
  }
  scheduleReconnectError(io);
  requestReconnect(io);
}

// Refresh route-specific data after reconnect so UI matches server state.
async function syncCurrentRoute() {
  const routeName = router?.currentRoute?.value?.name;
  if (!routeName) {
    return;
  }

  switch (routeName) {
    case ROUTE_NAME_GAME:
    case ROUTE_NAME_VS_AI: {
      const authStore = useAuthStore();
      const gameStore = useGameStore();
      const gameHistoryStore = useGameHistoryStore();
      await authStore.requestStatus(true);
      const gameId = Number(router.currentRoute.value.params.gameId);
      if (!Number.isInteger(gameId)) {
        router.push({ name: ROUTE_NAME_HOME });
        return;
      }

      const gameStateIndex = gameHistoryStore.currentGameStateIndex;
      const response = await gameStore.requestGameState(gameId, gameStateIndex);
      if (response?.victory?.gameOver && response.game.rematchGame) {
        await gameStore.requestGameState(response.game.rematchGame, -1, null, true);
        router.push({ name: routeName, params: { gameId: response.game.rematchGame } });
      }
      return;
    }
    case ROUTE_NAME_SPECTATE: {
      const gameStore = useGameStore();
      const gameHistoryStore = useGameHistoryStore();
      const gameId = Number(router.currentRoute.value.params.gameId);
      if (!Number.isInteger(gameId)) {
        router.push({ name: ROUTE_NAME_HOME });
        return;
      }

      return gameStore.requestSpectate(gameId, gameHistoryStore.currentGameStateIndex);
    }
    case ROUTE_NAME_LOBBY: {
      const authStore = useAuthStore();
      const gameStore = useGameStore();
      await authStore.requestStatus(true);
      const gameId = Number(router.currentRoute.value.params.gameId);
      if (!Number.isInteger(gameId)) {
        router.push({ name: ROUTE_NAME_HOME });
        return;
      }

      const response = await gameStore.requestSubscribe(gameId);
      if (response?.game?.status === 'STARTED') {
        router.push({ name: ROUTE_NAME_GAME, params: { gameId } });
      }
      return;
    }
    case ROUTE_NAME_HOME: {
      const gameListStore = useGameListStore();
      return gameListStore.requestGameList();
    }
    default:
      return;
  }
}

export async function handleConnect() {
  if (!hasActivePinia()) {
    return;
  }
  try {
    // Connection restored; clear reconnect UI and resync route state.
    clearReconnectError();
    await syncCurrentRoute();
  } catch (err) {
    // Keep socket lifecycle handlers non-fatal.
    console.warn('Socket re-sync failed after reconnect:', err);
  }
}

export function handleDisconnect(io, reason) {
  if (!hasActivePinia() || !routeRequiresSocket()) {
    return;
  }

  try {
    const isIntentionalManualDisconnect = reason === 'io client disconnect';
    if (isIntentionalManualDisconnect) {
      // `io client disconnect` is emitted when disconnect is requested by the client.
      clearReconnectError();
      return;
    }

    // Any non-manual disconnect on socket-required routes should recover automatically.
    console.warn('Socket disconnected:', reason);
    handleDisconnectedState(io);
  } catch (err) {
    console.warn('Socket disconnect handler failed:', err);
  }
}

export function handleConnectError(io, err) {
  if (!hasActivePinia() || !routeRequiresSocket()) {
    return;
  }

  try {
    // Connection errors use the same recovery path as disconnect events.
    console.warn('Socket connection error:', err?.message ?? err);
    handleDisconnectedState(io);
  } catch (handlerError) {
    console.warn('Socket connect_error handler failed:', handlerError);
  }
}
