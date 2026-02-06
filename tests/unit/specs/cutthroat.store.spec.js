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
    players: [],
  };
  return {
    type: 'state',
    version,
    seat: 1,
    status: 1,
    player_view: playerView,
    spectator_view: {
      ...playerView,
      deck_count: 0,
    },
    tokenlog: 'V1 CUTTHROAT3P DEALER P0 DECK AC ENDDECK',
    legal_actions: [ { type: 'Draw' } ],
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
    ws.emitMessage(buildStatePayload(3));

    expect(store.version).toBe(3);
    expect(store.seat).toBe(1);
    expect(store.status).toBe(1);
    expect(store.legalActions).toEqual([ { type: 'Draw' } ]);
    expect(store.lobby.seats[0].username).toBe('avi');
    expect(store.tokenlog).toBe('V1 CUTTHROAT3P DEALER P0 DECK AC ENDDECK');
  });

  it('only uses player_view for local player state', () => {
    const store = useCutthroatStore();
    store.connectWs(42);
    const [ ws ] = FakeWebSocket.instances;
    const payload = buildStatePayload(5);
    payload.player_view = null;
    payload.public_view = {
      seat: 1,
      turn: 1,
      phase: { type: 'Main' },
      deck_count: 999,
      scrap: [],
      players: [
        {
          seat: 1,
          hand: [ { type: 'Hidden' } ],
          points: [],
          royals: [],
          frozen: [],
        },
      ],
    };
    ws.emitMessage(payload);

    expect(store.playerView).toBeNull();
    expect(store.spectatorView.deck_count).toBe(0);
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

  it('handles scrap straighten websocket updates', () => {
    const store = useCutthroatStore();
    store.connectWs(42);
    const [ ws ] = FakeWebSocket.instances;

    ws.emitMessage({
      type: 'scrap_straighten',
      game_id: 42,
      straightened: true,
      actor_seat: 1,
    });
    expect(store.isScrapStraightened).toBe(true);

    ws.emitMessage({
      type: 'scrap_straighten',
      game_id: 42,
      straightened: false,
      actor_seat: 2,
    });
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
    ws.emitMessage(buildStatePayload(8));

    await expect(actionPromise).resolves.toBeUndefined();
    expect(store.version).toBe(8);
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
});
