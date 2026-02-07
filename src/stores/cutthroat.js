import { defineStore } from 'pinia';
import { ref } from 'vue';
import { useCapabilitiesStore } from '@/stores/capabilities';
import { resolveCutthroatHttpPath, resolveCutthroatWsUrl } from '@/util/cutthroat-url';

const ACTION_ACK_TIMEOUT_MS = 5000;
const WS_RECONNECT_INITIAL_DELAY_MS = 1000;
const WS_RECONNECT_MAX_DELAY_MS = 5000;

function createHttpError(message, status) {
  const err = new Error(`${message}: ${status}`);
  err.status = status;
  return err;
}

function isObject(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function isFiniteNumber(value) {
  return typeof value === 'number' && Number.isFinite(value);
}

function isStringArray(value) {
  return Array.isArray(value) && value.every((entry) => typeof entry === 'string');
}

function isValidPublicCard(card) {
  if (card === 'Hidden') {return true;}
  return isObject(card) && typeof card.Known === 'string';
}

function isValidPointStack(stack) {
  return isObject(stack)
    && typeof stack.base === 'string'
    && isFiniteNumber(stack.controller)
    && isStringArray(stack.jacks);
}

function isValidRoyalStack(stack) {
  return isObject(stack)
    && typeof stack.base === 'string'
    && isFiniteNumber(stack.controller)
    && isStringArray(stack.jokers);
}

function isValidPlayerView(player) {
  return isObject(player)
    && isFiniteNumber(player.seat)
    && Array.isArray(player.hand)
    && player.hand.every((card) => isValidPublicCard(card))
    && Array.isArray(player.points)
    && player.points.every((stack) => isValidPointStack(stack))
    && Array.isArray(player.royals)
    && player.royals.every((stack) => isValidRoyalStack(stack))
    && isStringArray(player.frozen);
}

function isValidPublicView(view) {
  return isObject(view)
    && isFiniteNumber(view.seat)
    && isFiniteNumber(view.turn)
    && isObject(view.phase)
    && typeof view.phase.type === 'string'
    && isFiniteNumber(view.deck_count)
    && isStringArray(view.scrap)
    && Array.isArray(view.players)
    && view.players.every((player) => isValidPlayerView(player));
}

function isValidLobbySeat(seat) {
  return isObject(seat)
    && isFiniteNumber(seat.seat)
    && isFiniteNumber(seat.user_id)
    && typeof seat.username === 'string'
    && typeof seat.ready === 'boolean';
}

function isValidLobbyView(view) {
  return isObject(view)
    && Array.isArray(view.seats)
    && view.seats.every((seat) => isValidLobbySeat(seat));
}

function isValidGameStatePayload(payload) {
  return isObject(payload)
    && isFiniteNumber(payload.version)
    && isFiniteNumber(payload.seat)
    && isFiniteNumber(payload.status)
    && isValidPublicView(payload.player_view)
    && isValidPublicView(payload.spectator_view)
    && Array.isArray(payload.legal_actions)
    && isValidLobbyView(payload.lobby)
    && isStringArray(payload.log_tail)
    && typeof payload.tokenlog === 'string'
    && typeof payload.is_spectator === 'boolean'
    && isStringArray(payload.spectating_usernames)
    && typeof payload.scrap_straightened === 'boolean';
}

function isValidLobbySummary(lobbyEntry) {
  return isObject(lobbyEntry)
    && isFiniteNumber(lobbyEntry.id)
    && typeof lobbyEntry.name === 'string'
    && isFiniteNumber(lobbyEntry.seat_count)
    && isFiniteNumber(lobbyEntry.ready_count)
    && isFiniteNumber(lobbyEntry.status);
}

function isValidSpectatableGame(gameEntry) {
  return isObject(gameEntry)
    && isFiniteNumber(gameEntry.id)
    && typeof gameEntry.name === 'string'
    && isFiniteNumber(gameEntry.seat_count)
    && isFiniteNumber(gameEntry.status)
    && isStringArray(gameEntry.spectating_usernames);
}

function isValidLobbyMessage(payload) {
  return isObject(payload)
    && payload.type === 'lobbies'
    && isFiniteNumber(payload.version)
    && Array.isArray(payload.lobbies)
    && payload.lobbies.every((entry) => isValidLobbySummary(entry))
    && Array.isArray(payload.spectatable_games)
    && payload.spectatable_games.every((entry) => isValidSpectatableGame(entry));
}

function isValidErrorMessage(payload) {
  return isObject(payload)
    && payload.type === 'error'
    && isFiniteNumber(payload.code)
    && typeof payload.message === 'string';
}

function protocolError(reason) {
  return `Cutthroat protocol violation: ${reason}`;
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
  const lobbyVersion = ref(0);
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

  function failGameProtocol(ws, reason) {
    const message = protocolError(reason);
    setLastError({ code: 1002, message });
    gameSocketShouldReconnect = false;
    activeGameSocketId = null;
    activeGameSocket = null;
    activeGameSocketSpectateIntent = false;
    clearGameReconnectTimer();
    gameSocketReconnectDelayMs = WS_RECONNECT_INITIAL_DELAY_MS;
    if (socket.value === ws) {
      socket.value = null;
    }
    rejectPendingAction(new Error(message));
    try {
      ws.close();
    } catch (_) {
      // ignore
    }
  }

  function failLobbyProtocol(ws, reason) {
    const message = protocolError(reason);
    setLastError({ code: 1002, message });
    lobbySocketShouldReconnect = false;
    activeLobbySocket = null;
    clearLobbyReconnectTimer();
    lobbySocketReconnectDelayMs = WS_RECONNECT_INITIAL_DELAY_MS;
    if (lobbySocket.value === ws) {
      lobbySocket.value = null;
    }
    try {
      ws.close();
    } catch (_) {
      // ignore
    }
  }

  function updateFromPayload(payload) {
    if (
      typeof payload.version === 'number'
      && typeof version.value === 'number'
      && payload.version < version.value
    ) {
      return;
    }
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
    isSpectator.value = payload.is_spectator;
    playerView.value = isSpectator.value
      ? payload.spectator_view
      : payload.player_view;
    spectatorView.value = payload.spectator_view;
    legalActions.value = payload.legal_actions;
    lobby.value = payload.lobby;
    spectatingUsers.value = payload.spectating_usernames;
    logTail.value = payload.log_tail;
    tokenlog.value = payload.tokenlog;
    isScrapStraightened.value = payload.scrap_straightened;
    lastEvent.value = playerView.value?.last_event ?? null;
  }

  async function fetchState(id, { spectateIntent = false } = {}) {
    ensureCutthroatAvailable();
    isScrapStraightened.value = false;
    if (gameId.value !== id) {
      version.value = 0;
    }
    const statePath = spectateIntent ? `/cutthroat/api/v1/games/${id}/spectate/state` : `/cutthroat/api/v1/games/${id}/state`;
    const res = await fetch(resolveCutthroatHttpPath(statePath), {
      credentials: 'include',
    });
    if (!res.ok) {
      throw createHttpError('Failed to fetch state', res.status);
    }
    const data = await res.json();
    if (!isValidGameStatePayload(data)) {
      const message = protocolError('invalid HTTP game state payload');
      setLastError({ code: 1002, message });
      throw new Error(message);
    }
    gameId.value = id;
    updateFromPayload(data);
    return data;
  }

  async function createGame() {
    ensureCutthroatAvailable();
    const res = await fetch(resolveCutthroatHttpPath('/cutthroat/api/v1/games'), {
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
    const res = await fetch(resolveCutthroatHttpPath(`/cutthroat/api/v1/games/${id}/join`), {
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
    const res = await fetch(resolveCutthroatHttpPath(`/cutthroat/api/v1/games/${id}/leave`), {
      method: 'POST',
      credentials: 'include',
    });
    if (!res.ok) {
      throw createHttpError('Failed to leave game', res.status);
    }
  }

  async function rematchGame(id) {
    ensureCutthroatAvailable();
    const res = await fetch(resolveCutthroatHttpPath(`/cutthroat/api/v1/games/${id}/rematch`), {
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
    const res = await fetch(resolveCutthroatHttpPath(`/cutthroat/api/v1/games/${id}/ready`), {
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
    const res = await fetch(resolveCutthroatHttpPath(`/cutthroat/api/v1/games/${id}/start`), {
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
    if (gameId.value !== id) {
      version.value = 0;
      gameId.value = id;
    }
    if (replace) {
      disconnectWs();
    }
    clearGameReconnectTimer();
    gameSocketShouldReconnect = true;
    activeGameSocketId = id;
    activeGameSocketSpectateIntent = spectateIntent;
    isScrapStraightened.value = false;
    const wsPath = spectateIntent ? `/cutthroat/ws/games/${id}/spectate` : `/cutthroat/ws/games/${id}`;
    const ws = new WebSocket(resolveCutthroatWsUrl(wsPath));
    activeGameSocket = ws;
    ws.onopen = () => {
      gameSocketReconnectDelayMs = WS_RECONNECT_INITIAL_DELAY_MS;
      clearLastError();
    };
    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data);
        if (!isObject(msg) || typeof msg.type !== 'string') {
          failGameProtocol(ws, 'invalid game WS message envelope');
          return;
        }
        if (msg.type === 'state') {
          if (!isObject(msg.state) || !isValidGameStatePayload(msg.state)) {
            failGameProtocol(ws, 'invalid game state payload');
            return;
          }
          updateFromPayload(msg.state);
          clearLastError();
          return;
        }
        if (msg.type === 'error') {
          if (!isValidErrorMessage(msg)) {
            failGameProtocol(ws, 'invalid game error payload');
            return;
          }
          const err = new Error(msg.message);
          err.status = msg.code;
          setLastError({
            code: msg.code,
            message: msg.message,
          });
          rejectPendingAction(err);
          return;
        }
        failGameProtocol(ws, `unexpected game message type "${msg.type}"`);
      } catch (_) {
        failGameProtocol(ws, 'non-JSON game WS payload');
      }
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
    const ws = new WebSocket(resolveCutthroatWsUrl('/cutthroat/ws/lobbies'));
    activeLobbySocket = ws;
    ws.onopen = () => {
      lobbySocketReconnectDelayMs = WS_RECONNECT_INITIAL_DELAY_MS;
      clearLastError();
    };
    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data);
        if (!isObject(msg) || typeof msg.type !== 'string') {
          failLobbyProtocol(ws, 'invalid lobby WS message envelope');
          return;
        }
        if (msg.type === 'lobbies') {
          if (!isValidLobbyMessage(msg)) {
            failLobbyProtocol(ws, 'invalid lobby payload');
            return;
          }
          if (msg.version < lobbyVersion.value) {
            return;
          }
          lobbyVersion.value = msg.version;
          lobbies.value = msg.lobbies;
          spectateGames.value = msg.spectatable_games;
          return;
        }
        if (msg.type === 'error') {
          if (!isValidErrorMessage(msg)) {
            failLobbyProtocol(ws, 'invalid lobby error payload');
            return;
          }
          setLastError({
            code: msg.code,
            message: msg.message,
          });
          return;
        }
        failLobbyProtocol(ws, `unexpected lobby message type "${msg.type}"`);
      } catch (_) {
        failLobbyProtocol(ws, 'non-JSON lobby WS payload');
      }
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
    lobbyVersion.value = 0;
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
    const res = await fetch(resolveCutthroatHttpPath(`/cutthroat/api/v1/games/${gameId.value}/action`), {
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
    if (!isValidGameStatePayload(data)) {
      const message = protocolError('invalid HTTP action response payload');
      setLastError({ code: 1002, message });
      throw new Error(message);
    }
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
