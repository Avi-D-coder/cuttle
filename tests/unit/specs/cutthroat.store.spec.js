import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useCutthroatStore } from '@/stores/cutthroat';
import { useCapabilitiesStore } from '@/stores/capabilities';

class FakeWebSocket {
  static OPEN = 1;
  static CLOSED = 3;
  static instances = [];

  constructor(url) {
    this.url = url;
    this.readyState = FakeWebSocket.OPEN;
    this.sent = [];
    this.onmessage = null;
    this.onclose = null;
    FakeWebSocket.instances.push(this);
  }

  send(payload) {
    this.sent.push(payload);
  }

  close() {
    this.readyState = FakeWebSocket.CLOSED;
    if (this.onclose) {
      this.onclose();
    }
  }

  emitMessage(payload) {
    if (this.onmessage) {
      this.onmessage({ data: JSON.stringify(payload) });
    }
  }
}

function buildStatePayload(version = 1) {
  const playerView = {
    seat: 1,
    turn: 1,
    phase: { type: 'Main' },
    deck_count: 10,
    scrap: [],
    players: [
      {
        seat: 0,
        hand: [ 'Hidden' ],
        points: [],
        royals: [],
        frozen: [],
      },
      {
        seat: 1,
        hand: [ { Known: '9C' } ],
        points: [],
        royals: [],
        frozen: [],
      },
      {
        seat: 2,
        hand: [ 'Hidden' ],
        points: [],
        royals: [],
        frozen: [],
      },
    ],
  };
  return {
    version,
    seat: 1,
    status: 1,
    is_spectator: false,
    player_view: playerView,
    spectator_view: {
      ...playerView,
      deck_count: 0,
    },
    tokenlog: 'V1 CUTTHROAT3P DEALER P0 DECK AC ENDDECK',
    log_tail: [],
    legal_actions: [ { type: 'Draw' } ],
    spectating_usernames: [],
    scrap_straightened: false,
    lobby: {
      seats: [ { seat: 1, user_id: 10, username: 'avi', ready: true } ],
    },
  };
}

describe('cutthroat store websocket behavior', () => {
  beforeEach(() => {
    FakeWebSocket.instances = [];
    setActivePinia(createPinia());
    vi.stubGlobal('WebSocket', FakeWebSocket);
    vi.stubGlobal('window', {
      location: {
        protocol: 'http:',
        host: 'localhost:8080',
      },
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it('handles ws state payload updates', () => {
    const store = useCutthroatStore();
    store.connectWs(42);
    const [ ws ] = FakeWebSocket.instances;
    ws.emitMessage({ type: 'state', state: buildStatePayload(3) });

    expect(store.version).toBe(3);
    expect(store.seat).toBe(1);
    expect(store.status).toBe(1);
    expect(store.legalActions).toEqual([ { type: 'Draw' } ]);
    expect(store.lobby.seats[0].username).toBe('avi');
    expect(store.tokenlog).toBe('V1 CUTTHROAT3P DEALER P0 DECK AC ENDDECK');
  });

  it('uses spectator_view when spectator flag is true', () => {
    const store = useCutthroatStore();
    store.connectWs(42);
    const [ ws ] = FakeWebSocket.instances;
    const payload = buildStatePayload(5);
    payload.is_spectator = true;
    payload.player_view.deck_count = 99;
    payload.spectator_view.deck_count = 0;
    ws.emitMessage({ type: 'state', state: payload });

    expect(store.isSpectator).toBe(true);
    expect(store.playerView.deck_count).toBe(0);
    expect(store.spectatorView.deck_count).toBe(0);
  });

  it('ignores stale websocket game state versions', () => {
    const store = useCutthroatStore();
    store.connectWs(42);
    const [ ws ] = FakeWebSocket.instances;

    ws.emitMessage({ type: 'state', state: buildStatePayload(5) });
    ws.emitMessage({ type: 'state', state: buildStatePayload(4) });

    expect(store.version).toBe(5);
  });

  it('fails protocol on malformed game state message and does not reconnect automatically', () => {
    vi.useFakeTimers();
    try {
      const store = useCutthroatStore();
      store.connectWs(42);
      const [ ws ] = FakeWebSocket.instances;

      ws.emitMessage({ type: 'state' });
      vi.advanceTimersByTime(5000);

      expect(store.lastError.message).toContain('Cutthroat protocol violation');
      expect(FakeWebSocket.instances).toHaveLength(1);
      expect(ws.readyState).toBe(FakeWebSocket.CLOSED);
    } finally {
      vi.useRealTimers();
    }
  });

  it('handles ws error payload updates', () => {
    const store = useCutthroatStore();
    store.connectWs(42);
    const [ ws ] = FakeWebSocket.instances;
    ws.emitMessage({ type: 'error', code: 409, message: 'conflict' });

    expect(store.lastError).toEqual({
      code: 409,
      message: 'conflict',
    });
  });

  it('derives scrap straighten state from state payload', () => {
    const store = useCutthroatStore();
    store.connectWs(42);
    const [ ws ] = FakeWebSocket.instances;

    const first = buildStatePayload(2);
    first.scrap_straightened = true;
    ws.emitMessage({ type: 'state', state: first });
    expect(store.isScrapStraightened).toBe(true);

    const second = buildStatePayload(3);
    second.scrap_straightened = false;
    ws.emitMessage({ type: 'state', state: second });
    expect(store.isScrapStraightened).toBe(false);
  });

  it('sendAction rejects when websocket receives error', async () => {
    const store = useCutthroatStore();
    store.connectWs(99);
    const [ ws ] = FakeWebSocket.instances;
    store.version = 4;

    const actionPromise = store.sendAction({ type: 'Draw' });
    const [ sentAction ] = ws.sent;
    const sent = JSON.parse(sentAction);
    expect(sent.expected_version).toBe(4);
    expect(sent.action.type).toBe('Draw');

    ws.emitMessage({ type: 'error', code: 400, message: 'illegal action' });

    await expect(actionPromise).rejects.toThrow('illegal action');
    expect(store.lastError.message).toBe('illegal action');
  });

  it('sendAction resolves when websocket receives next state version', async () => {
    const store = useCutthroatStore();
    store.connectWs(99);
    const [ ws ] = FakeWebSocket.instances;
    store.version = 7;

    const actionPromise = store.sendAction({ type: 'Draw' });
    ws.emitMessage({ type: 'state', state: buildStatePayload(8) });

    await expect(actionPromise).resolves.toBeUndefined();
    expect(store.version).toBe(8);
  });

  it('sendAction rejects when websocket receives malformed state payload', async () => {
    const store = useCutthroatStore();
    store.connectWs(99);
    const [ ws ] = FakeWebSocket.instances;
    store.version = 7;

    const actionPromise = store.sendAction({ type: 'Draw' });
    ws.emitMessage({ type: 'state', state: { version: 8 } });

    await expect(actionPromise).rejects.toThrow('Cutthroat protocol violation');
    expect(store.lastError.message).toContain('Cutthroat protocol violation');
  });

  it('sendScrapStraighten sends websocket message', () => {
    const store = useCutthroatStore();
    store.connectWs(99);
    const [ ws ] = FakeWebSocket.instances;

    store.sendScrapStraighten();

    expect(ws.sent).toHaveLength(1);
    expect(JSON.parse(ws.sent[0])).toEqual({
      type: 'scrap_straighten',
    });
  });

  it('connects spectator websocket path when requested', () => {
    const store = useCutthroatStore();
    store.connectWs(42, { spectateIntent: true });
    const [ ws ] = FakeWebSocket.instances;
    expect(ws.url).toContain('/cutthroat/ws/games/42/spectate');
  });

  it('resets version when switching to another game websocket', () => {
    const store = useCutthroatStore();
    store.gameId = 10;
    store.version = 20;

    store.connectWs(11);
    const [ ws ] = FakeWebSocket.instances;
    ws.emitMessage({ type: 'state', state: buildStatePayload(3) });

    expect(store.gameId).toBe(11);
    expect(store.version).toBe(3);
    expect(store.playerView).not.toBeNull();
  });

  it('routes lobby websocket through vite proxy when app is served on localhost:1337', () => {
    vi.stubGlobal('window', {
      location: {
        protocol: 'http:',
        host: 'localhost:1337',
        hostname: 'localhost',
        port: '1337',
      },
    });
    const store = useCutthroatStore();
    store.connectLobbyWs();
    const [ ws ] = FakeWebSocket.instances;
    expect(ws.url).toBe('ws://localhost:8080/cutthroat/ws/lobbies');
  });

  it('reconnects game websocket after unexpected close', () => {
    vi.useFakeTimers();
    try {
      const store = useCutthroatStore();
      store.connectWs(42);
      const [ ws ] = FakeWebSocket.instances;

      ws.close();
      vi.runOnlyPendingTimers();

      expect(FakeWebSocket.instances).toHaveLength(2);
      expect(FakeWebSocket.instances[1].url).toContain('/cutthroat/ws/games/42');
    } finally {
      vi.useRealTimers();
    }
  });

  it('does not reconnect game websocket after manual disconnect', () => {
    vi.useFakeTimers();
    try {
      const store = useCutthroatStore();
      store.connectWs(42);

      store.disconnectWs();
      vi.advanceTimersByTime(5000);

      expect(FakeWebSocket.instances).toHaveLength(1);
    } finally {
      vi.useRealTimers();
    }
  });

  it('reconnects lobby websocket after unexpected close', () => {
    vi.useFakeTimers();
    try {
      const store = useCutthroatStore();
      store.connectLobbyWs();
      const [ ws ] = FakeWebSocket.instances;

      ws.close();
      vi.runOnlyPendingTimers();

      expect(FakeWebSocket.instances).toHaveLength(2);
      expect(FakeWebSocket.instances[1].url).toContain('/cutthroat/ws/lobbies');
    } finally {
      vi.useRealTimers();
    }
  });

  it('does not connect lobby websocket when cutthroat is unavailable', () => {
    const capabilitiesStore = useCapabilitiesStore();
    capabilitiesStore.cutthroatAvailability = 'unavailable';
    const store = useCutthroatStore();

    store.connectLobbyWs();

    expect(FakeWebSocket.instances).toHaveLength(0);
  });

  it('stores lobby spectatable games from lobby websocket payload', () => {
    const store = useCutthroatStore();
    store.connectLobbyWs();
    const [ ws ] = FakeWebSocket.instances;

    ws.emitMessage({
      type: 'lobbies',
      version: 1,
      lobbies: [ { id: 1, name: 'lobby', seat_count: 1, ready_count: 0, status: 0 } ],
      spectatable_games: [
        { id: 2, name: 'active', seat_count: 3, status: 1, spectating_usernames: [] },
      ],
    });

    expect(store.lobbies).toHaveLength(1);
    expect(store.spectateGames).toHaveLength(1);
    expect(store.spectateGames[0].id).toBe(2);
  });

  it('ignores stale lobby versions', () => {
    const store = useCutthroatStore();
    store.connectLobbyWs();
    const [ ws ] = FakeWebSocket.instances;

    ws.emitMessage({
      type: 'lobbies',
      version: 2,
      lobbies: [ { id: 10, name: 'new', seat_count: 1, ready_count: 0, status: 0 } ],
      spectatable_games: [],
    });
    ws.emitMessage({
      type: 'lobbies',
      version: 1,
      lobbies: [ { id: 9, name: 'old', seat_count: 1, ready_count: 0, status: 0 } ],
      spectatable_games: [],
    });

    expect(store.lobbies).toHaveLength(1);
    expect(store.lobbies[0].id).toBe(10);
  });

  it('fails protocol on malformed lobby payload and does not reconnect automatically', () => {
    vi.useFakeTimers();
    try {
      const store = useCutthroatStore();
      store.connectLobbyWs();
      const [ ws ] = FakeWebSocket.instances;

      ws.emitMessage({
        type: 'lobbies',
        version: '2',
        lobbies: [],
        spectatable_games: [],
      });
      vi.advanceTimersByTime(5000);

      expect(store.lastError.message).toContain('Cutthroat protocol violation');
      expect(FakeWebSocket.instances).toHaveLength(1);
      expect(ws.readyState).toBe(FakeWebSocket.CLOSED);
    } finally {
      vi.useRealTimers();
    }
  });
});

describe('cutthroat store http methods', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.stubGlobal('fetch', vi.fn());
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it('rematchGame posts and returns rematch id', async () => {
    const store = useCutthroatStore();
    fetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ id: 321 }),
    });

    await expect(store.rematchGame(77)).resolves.toBe(321);
    expect(fetch).toHaveBeenCalledWith('/cutthroat/api/v1/games/77/rematch', {
      method: 'POST',
      credentials: 'include',
    });
  });

  it('createGame posts with no custom name payload', async () => {
    const store = useCutthroatStore();
    fetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ id: 654 }),
    });

    await expect(store.createGame()).resolves.toBe(654);
    expect(fetch).toHaveBeenCalledWith('/cutthroat/api/v1/games', {
      method: 'POST',
      credentials: 'include',
    });
  });

  it('leaveGame posts to leave endpoint', async () => {
    const store = useCutthroatStore();
    fetch.mockResolvedValue({ ok: true });

    await expect(store.leaveGame(42)).resolves.toBeUndefined();
    expect(fetch).toHaveBeenCalledWith('/cutthroat/api/v1/games/42/leave', {
      method: 'POST',
      credentials: 'include',
    });
  });

  it('routes cutthroat http calls through vite proxy when app is served on localhost:1337', async () => {
    vi.stubGlobal('window', {
      location: {
        protocol: 'http:',
        host: 'localhost:1337',
        hostname: 'localhost',
        port: '1337',
      },
    });
    const store = useCutthroatStore();
    fetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ id: 999 }),
    });

    await expect(store.createGame()).resolves.toBe(999);
    expect(fetch).toHaveBeenCalledWith('http://localhost:8080/cutthroat/api/v1/games', {
      method: 'POST',
      credentials: 'include',
    });
  });

  it('fetchState expects bare GameStateResponse payloads', async () => {
    const store = useCutthroatStore();
    fetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(buildStatePayload(6)),
    });

    await store.fetchState(77);

    expect(store.version).toBe(6);
    expect(store.playerView).not.toBeNull();
    expect(store.status).toBe(1);
  });

  it('fetchState includes replay index query in spectate mode', async () => {
    const store = useCutthroatStore();
    fetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(buildStatePayload(9)),
    });

    await store.fetchState(77, {
      spectateIntent: true,
      gameStateIndex: 3,
    });

    expect(fetch).toHaveBeenCalledWith('/cutthroat/api/v1/games/77/spectate/state?gameStateIndex=3', {
      credentials: 'include',
    });
  });

  it('fetchState applies older versions for indexed replay snapshots', async () => {
    const store = useCutthroatStore();
    fetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve(buildStatePayload(6)),
    });
    fetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve(buildStatePayload(2)),
    });

    await store.fetchState(77, {
      spectateIntent: true,
      gameStateIndex: 4,
    });
    await store.fetchState(77, {
      spectateIntent: true,
      gameStateIndex: 0,
    });

    expect(store.version).toBe(2);
  });

  it('fetchState throws when payload does not match game state contract', async () => {
    const store = useCutthroatStore();
    fetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ version: 6 }),
    });

    await expect(store.fetchState(77)).rejects.toThrow('Cutthroat protocol violation');
    expect(store.lastError.message).toContain('Cutthroat protocol violation');
  });
});
