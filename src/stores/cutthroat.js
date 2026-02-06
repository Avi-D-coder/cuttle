import { defineStore } from 'pinia';
import { ref } from 'vue';
import { useCapabilitiesStore } from '@/stores/capabilities';

const ACTION_ACK_TIMEOUT_MS = 5000;
const WS_RECONNECT_INITIAL_DELAY_MS = 1000;
const WS_RECONNECT_MAX_DELAY_MS = 5000;

function createHttpError(message, status) {
  const err = new Error(`${message}: ${status}`);
  err.status = status;
  return err;
}

export const useCutthroatStore = defineStore('cutthroat', () => {
  const capabilitiesStore = useCapabilitiesStore();
  const gameId = ref(null);
  const seat = ref(null);
  const version = ref(0);
  const status = ref(null);
  const playerView = ref(null);
  const spectatorView = ref(null);
  const legalActions = ref([]);
  const isSpectator = ref(false);
  const lobby = ref({ seats: [] });
  const spectatingUsers = ref([]);
  const logTail = ref([]);
  const tokenlog = ref('');
  const lastEvent = ref(null);
  const socket = ref(null);
  const lobbySocket = ref(null);
  const lobbies = ref([]);
  const spectateGames = ref([]);
  const lastError = ref(null);
  const pendingAction = ref(null);
  const isScrapStraightened = ref(false);
  let gameSocketShouldReconnect = false;
  let gameSocketReconnectTimer = null;
  let gameSocketReconnectDelayMs = WS_RECONNECT_INITIAL_DELAY_MS;
  let activeGameSocketId = null;
  let activeGameSocket = null;
  let activeGameSocketSpectateIntent = false;
  let lobbySocketShouldReconnect = false;
  let lobbySocketReconnectTimer = null;
  let lobbySocketReconnectDelayMs = WS_RECONNECT_INITIAL_DELAY_MS;
  let activeLobbySocket = null;

  function clearLastError() {
    lastError.value = null;
  }

  function ensureCutthroatAvailable() {
    if (capabilitiesStore.cutthroatAvailability !== 'unavailable') {return;}
    throw new Error('Cutthroat service is unavailable.');
  }

  function setLastError(error) {
    lastError.value = error;
  }

  function rejectPendingAction(err) {
    if (!pendingAction.value) {return;}
    clearTimeout(pendingAction.value.timeoutId);
    const { reject } = pendingAction.value;
    pendingAction.value = null;
    reject(err);
  }

  function resolvePendingAction() {
    if (!pendingAction.value) {return;}
    clearTimeout(pendingAction.value.timeoutId);
    const { resolve } = pendingAction.value;
    pendingAction.value = null;
    resolve();
  }

  function clearGameReconnectTimer() {
    if (!gameSocketReconnectTimer) {return;}
    clearTimeout(gameSocketReconnectTimer);
    gameSocketReconnectTimer = null;
  }

  function clearLobbyReconnectTimer() {
    if (!lobbySocketReconnectTimer) {return;}
    clearTimeout(lobbySocketReconnectTimer);
    lobbySocketReconnectTimer = null;
  }

  function scheduleGameReconnect() {
    if (gameSocketReconnectTimer || !gameSocketShouldReconnect || activeGameSocketId === null) {return;}
    gameSocketReconnectTimer = setTimeout(() => {
      gameSocketReconnectTimer = null;
      if (!gameSocketShouldReconnect || activeGameSocketId === null || socket.value) {return;}
      connectWs(activeGameSocketId, {
        replace: false,
        spectateIntent: activeGameSocketSpectateIntent,
      });
      gameSocketReconnectDelayMs = Math.min(gameSocketReconnectDelayMs * 2, WS_RECONNECT_MAX_DELAY_MS);
    }, gameSocketReconnectDelayMs);
  }

  function scheduleLobbyReconnect() {
    if (lobbySocketReconnectTimer || !lobbySocketShouldReconnect) {return;}
    lobbySocketReconnectTimer = setTimeout(() => {
      lobbySocketReconnectTimer = null;
      if (!lobbySocketShouldReconnect || lobbySocket.value) {return;}
      connectLobbyWs({ replace: false });
      lobbySocketReconnectDelayMs = Math.min(lobbySocketReconnectDelayMs * 2, WS_RECONNECT_MAX_DELAY_MS);
    }, lobbySocketReconnectDelayMs);
  }

  function updateFromPayload(payload) {
    if (!payload) {return;}
    if (
      pendingAction.value
      && typeof payload.version === 'number'
      && payload.version > pendingAction.value.expectedVersion
    ) {
      resolvePendingAction();
    }
    version.value = payload.version;
    seat.value = payload.seat;
    status.value = payload.status;
    isSpectator.value = Boolean(payload.is_spectator);
    playerView.value = isSpectator.value ? (payload.spectator_view ?? payload.player_view ?? null) : (payload.player_view ?? null);
    spectatorView.value = payload.spectator_view ?? null;
    legalActions.value = payload.legal_actions ?? [];
    lobby.value = payload.lobby ?? { seats: [] };
    spectatingUsers.value = payload.spectating_usernames ?? [];
    logTail.value = payload.log_tail ?? [];
    tokenlog.value = payload.tokenlog ?? '';
    lastEvent.value = playerView.value?.last_event ?? null;
  }

  async function fetchState(id, { spectateIntent = false } = {}) {
    ensureCutthroatAvailable();
    isScrapStraightened.value = false;
    const statePath = spectateIntent ? `/cutthroat/api/v1/games/${id}/spectate/state` : `/cutthroat/api/v1/games/${id}/state`;
    const res = await fetch(statePath, {
      credentials: 'include',
    });
    if (!res.ok) {
      throw createHttpError('Failed to fetch state', res.status);
    }
    const data = await res.json();
    gameId.value = id;
    updateFromPayload(data);
    return data;
  }

  async function createGame() {
    ensureCutthroatAvailable();
    const res = await fetch('/cutthroat/api/v1/games', {
      method: 'POST',
      credentials: 'include',
    });
    if (!res.ok) {
      throw createHttpError('Failed to create game', res.status);
    }
    const data = await res.json();
    return data.id;
  }

  async function joinGame(id) {
    ensureCutthroatAvailable();
    const res = await fetch(`/cutthroat/api/v1/games/${id}/join`, {
      method: 'POST',
      credentials: 'include',
    });
    if (!res.ok) {
      throw createHttpError('Failed to join game', res.status);
    }
    return res.json();
  }

  async function leaveGame(id) {
    ensureCutthroatAvailable();
    const res = await fetch(`/cutthroat/api/v1/games/${id}/leave`, {
      method: 'POST',
      credentials: 'include',
    });
    if (!res.ok) {
      throw createHttpError('Failed to leave game', res.status);
    }
  }

  async function rematchGame(id) {
    ensureCutthroatAvailable();
    const res = await fetch(`/cutthroat/api/v1/games/${id}/rematch`, {
      method: 'POST',
      credentials: 'include',
    });
    if (!res.ok) {
      throw createHttpError('Failed to rematch game', res.status);
    }
    const data = await res.json();
    return data.id;
  }

  async function setReady(id, ready = true) {
    ensureCutthroatAvailable();
    const res = await fetch(`/cutthroat/api/v1/games/${id}/ready`, {
      method: 'POST',
      headers: new Headers({
        'Content-Type': 'application/json',
      }),
      credentials: 'include',
      body: JSON.stringify({ ready }),
    });
    if (!res.ok) {
      throw createHttpError('Failed to set ready', res.status);
    }
  }

  async function startGame(id) {
    ensureCutthroatAvailable();
    const res = await fetch(`/cutthroat/api/v1/games/${id}/start`, {
      method: 'POST',
      credentials: 'include',
    });
    if (!res.ok) {
      throw createHttpError('Failed to start game', res.status);
    }
  }

  function connectWs(id, { replace = true, spectateIntent = false } = {}) {
    if (capabilitiesStore.cutthroatAvailability === 'unavailable') {return;}
    if (!Number.isInteger(id)) {return;}
    if (replace) {
      disconnectWs();
    }
    clearGameReconnectTimer();
    gameSocketShouldReconnect = true;
    activeGameSocketId = id;
    activeGameSocketSpectateIntent = spectateIntent;
    isScrapStraightened.value = false;
    const protocol = window.location.protocol === 'https:' ? 'wss' : 'ws';
    const wsPath = spectateIntent ? `/cutthroat/ws/games/${id}/spectate` : `/cutthroat/ws/games/${id}`;
    const ws = new WebSocket(`${protocol}://${window.location.host}${wsPath}`);
    activeGameSocket = ws;
    ws.onopen = () => {
      gameSocketReconnectDelayMs = WS_RECONNECT_INITIAL_DELAY_MS;
      clearLastError();
    };
    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data);
        if (msg.type === 'state') {
          updateFromPayload(msg.data ?? msg.state ?? msg);
          clearLastError();
        } else if (msg.type === 'scrap_straighten') {
          isScrapStraightened.value = Boolean(msg.straightened);
        } else if (msg.type === 'error') {
          const err = new Error(msg.message ?? 'WebSocket error');
          err.status = msg.code;
          setLastError({
            code: msg.code,
            message: msg.message ?? 'WebSocket error',
          });
          rejectPendingAction(err);
        }
      } catch (_) {/* ignore */}
    };
    ws.onclose = () => {
      if (activeGameSocket === ws) {
        activeGameSocket = null;
        socket.value = null;
        scheduleGameReconnect();
      }
      rejectPendingAction(new Error('WebSocket disconnected'));
    };
    socket.value = ws;
  }

  function disconnectWs() {
    gameSocketShouldReconnect = false;
    activeGameSocketId = null;
    activeGameSocket = null;
    activeGameSocketSpectateIntent = false;
    clearGameReconnectTimer();
    gameSocketReconnectDelayMs = WS_RECONNECT_INITIAL_DELAY_MS;
    if (socket.value) {
      socket.value.close();
      socket.value = null;
    }
  }

  function connectLobbyWs({ replace = true } = {}) {
    if (capabilitiesStore.cutthroatAvailability === 'unavailable') {return;}
    if (replace) {
      disconnectLobbyWs();
    }
    clearLobbyReconnectTimer();
    lobbySocketShouldReconnect = true;
    if (lobbySocket.value) {return;}
    const protocol = window.location.protocol === 'https:' ? 'wss' : 'ws';
    const ws = new WebSocket(`${protocol}://${window.location.host}/cutthroat/ws/lobbies`);
    activeLobbySocket = ws;
    ws.onopen = () => {
      lobbySocketReconnectDelayMs = WS_RECONNECT_INITIAL_DELAY_MS;
      clearLastError();
    };
    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data);
        if (msg.type === 'lobbies') {
          lobbies.value = msg.lobbies ?? [];
          spectateGames.value = msg.spectatable_games ?? [];
        } else if (msg.type === 'error') {
          setLastError({
            code: msg.code,
            message: msg.message ?? 'WebSocket error',
          });
        }
      } catch (_) {/* ignore */}
    };
    ws.onclose = () => {
      if (activeLobbySocket === ws) {
        activeLobbySocket = null;
        lobbySocket.value = null;
        scheduleLobbyReconnect();
      }
    };
    lobbySocket.value = ws;
  }

  function disconnectLobbyWs() {
    lobbySocketShouldReconnect = false;
    activeLobbySocket = null;
    clearLobbyReconnectTimer();
    lobbySocketReconnectDelayMs = WS_RECONNECT_INITIAL_DELAY_MS;
    if (lobbySocket.value) {
      lobbySocket.value.close();
      lobbySocket.value = null;
    }
    lobbies.value = [];
    spectateGames.value = [];
  }

  async function sendAction(action) {
    ensureCutthroatAvailable();
    if (!action) {return;}
    if (socket.value && socket.value.readyState === WebSocket.OPEN) {
      if (pendingAction.value) {
        throw new Error('Another action is already pending.');
      }
      const expectedVersion = version.value;
      await new Promise((resolve, reject) => {
        const timeoutId = setTimeout(() => {
          rejectPendingAction(new Error('Timed out waiting for game update.'));
        }, ACTION_ACK_TIMEOUT_MS);
        pendingAction.value = {
          expectedVersion,
          resolve,
          reject,
          timeoutId,
        };
        try {
          socket.value.send(JSON.stringify({
            type: 'action',
            expected_version: expectedVersion,
            action,
          }));
        } catch (err) {
          rejectPendingAction(err instanceof Error ? err : new Error('Failed to send WebSocket action.'));
        }
      });
      return;
    }
    const res = await fetch(`/cutthroat/api/v1/games/${gameId.value}/action`, {
      method: 'POST',
      headers: new Headers({
        'Content-Type': 'application/json',
      }),
      credentials: 'include',
      body: JSON.stringify({
        expected_version: version.value,
        action,
      }),
    });
    if (!res.ok) {
      throw createHttpError('Failed to send action', res.status);
    }
    const data = await res.json();
    updateFromPayload(data);
  }

  function sendScrapStraighten() {
    ensureCutthroatAvailable();
    if (!socket.value || socket.value.readyState !== WebSocket.OPEN) {
      throw new Error('WebSocket is not connected.');
    }
    socket.value.send(JSON.stringify({
      type: 'scrap_straighten',
    }));
  }

  return {
    gameId,
    seat,
    version,
    status,
    playerView,
    spectatorView,
    legalActions,
    isSpectator,
    lobby,
    spectatingUsers,
    logTail,
    tokenlog,
    lastEvent,
    lobbies,
    spectateGames,
    lastError,
    isScrapStraightened,
    clearLastError,
    fetchState,
    createGame,
    joinGame,
    leaveGame,
    rematchGame,
    setReady,
    startGame,
    connectWs,
    disconnectWs,
    connectLobbyWs,
    disconnectLobbyWs,
    sendAction,
    sendScrapStraighten,
  };
});
