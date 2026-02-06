function installMockCutthroatWs(win) {
  class MockWebSocket {
    static OPEN = 1;
    static CLOSED = 3;

    constructor(url) {
      this.url = url;
      this.readyState = MockWebSocket.OPEN;
      this.sent = [];
      this.onmessage = null;
      this.onclose = null;
      win.__cutthroatWsInstances.push(this);
    }

    send(payload) {
      this.sent.push(payload);
    }

    close() {
      this.readyState = MockWebSocket.CLOSED;
      if (this.onclose) {
        this.onclose();
      }
    }

    emit(payload) {
      if (this.onmessage) {
        this.onmessage({
          data: JSON.stringify(payload),
        });
      }
    }
  }

  win.__cutthroatWsInstances = [];
  win.WebSocket = MockWebSocket;
}

function getGameWs(win, gameId) {
  return win.__cutthroatWsInstances.find((entry) => entry.url.includes(`/cutthroat/ws/games/${gameId}`));
}

function known(token) {
  return {
    type: 'Known',
    data: token,
  };
}

function hidden() {
  return {
    type: 'Hidden',
  };
}

function buildRoyalEightPlayers(viewerSeat) {
  return [
    {
      seat: 0,
      hand: [ viewerSeat === 0 ? known('3C') : hidden() ],
      points: [],
      royals: [],
      frozen: [],
    },
    {
      seat: 1,
      hand: [ viewerSeat === 1 ? known('4C') : hidden() ],
      points: [],
      royals: [ { base: '8C', controller: 1, jokers: [] } ],
      frozen: [],
    },
    {
      seat: 2,
      hand: [ viewerSeat === 2 ? known('5C') : hidden() ],
      points: [],
      royals: [],
      frozen: [],
    },
  ];
}

function buildStartedState({
  gameId = 1,
  version = 1,
  seat = 0,
  isSpectator = false,
  turn = seat,
  phase = { type: 'Main' },
  legalActions = [],
  scrap = [],
  logTail = [ 'Game started.' ],
  tokenlog = 'V1 CUTTHROAT3P DEALER P0 DECK AC AD AH AS ENDDECK',
  players = null,
} = {}) {
  const playerView = {
    seat,
    turn,
    phase,
    deck_count: 40,
    scrap,
    players: players ?? [
      {
        seat: 0,
        hand: [ known('3C') ],
        points: [],
        royals: [],
        frozen: [],
      },
      {
        seat: 1,
        hand: [ hidden() ],
        points: [],
        royals: [],
        frozen: [],
      },
      {
        seat: 2,
        hand: [ hidden() ],
        points: [],
        royals: [],
        frozen: [],
      },
    ],
  };
  return {
    version,
    seat,
    status: 1,
    is_spectator: isSpectator,
    player_view: playerView,
    spectator_view: {
      ...playerView,
      deck_count: 0,
    },
    legal_actions: legalActions,
    log_tail: logTail,
    tokenlog,
    lobby: {
      seats: [
        { seat: 0, user_id: 100, username: 'avi', ready: true },
        { seat: 1, user_id: 101, username: 'op1', ready: true },
        { seat: 2, user_id: 102, username: 'op2', ready: true },
      ],
    },
    game_id: gameId,
  };
}

function buildLobbyState({ version = 1, seat = 0 } = {}) {
  const playerView = {
    seat,
    turn: seat,
    phase: { type: 'Main' },
    deck_count: 56,
    scrap: [],
    players: [],
  };
  return {
    version,
    seat,
    status: 0,
    player_view: playerView,
    spectator_view: {
      ...playerView,
      deck_count: 0,
    },
    legal_actions: [],
    lobby: {
      seats: [
        { seat: 0, user_id: 100, username: 'avi', ready: true },
        { seat: 1, user_id: 101, username: 'op1', ready: true },
        { seat: 2, user_id: 102, username: 'op2', ready: false },
      ],
    },
  };
}

describe('Cutthroat 3P smoke', () => {
  beforeEach(() => {
    cy.intercept('GET', '/api/user/status', {
      authenticated: true,
      id: 100,
      username: 'avi',
    });
  });

  it('auto-start transitions lobby to game route', () => {
    const gameId = 777;
    const lobbyState = buildLobbyState();
    const startedState = buildStartedState({
      gameId,
      version: 2,
      legalActions: [ { type: 'Draw' } ],
    });

    let stateCallCount = 0;
    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/state`, (req) => {
      stateCallCount += 1;
      req.reply(stateCallCount === 1 ? lobbyState : startedState);
    });
    cy.intercept('POST', `/cutthroat/api/v1/games/${gameId}/join`, { seat: 0 });

    cy.visit(`/cutthroat/lobby/${gameId}`, {
      onBeforeLoad(win) {
        installMockCutthroatWs(win);
      },
    });

    cy.contains('Cutthroat Lobby').should('be.visible');
    cy.window().should((win) => {
      expect(getGameWs(win, gameId)).to.exist;
    });
    cy.window().then((win) => {
      const ws = getGameWs(win, gameId);
      ws.emit({
        type: 'state',
        ...startedState,
      });
    });

    cy.url().should('include', `/cutthroat/game/${gameId}`);
    cy.get('[data-cy=cutthroat-deck]').should('be.visible');
  });

  it('deck click sends Draw action when legal', () => {
    const gameId = 801;
    const startedState = buildStartedState({
      gameId,
      legalActions: [ { type: 'Draw' } ],
    });

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/state`, startedState);
    cy.intercept('POST', `/cutthroat/api/v1/games/${gameId}/join`, { seat: 0 });

    cy.visit(`/cutthroat/game/${gameId}`, {
      onBeforeLoad(win) {
        installMockCutthroatWs(win);
      },
    });

    cy.get('[data-cy=cutthroat-history-log]').contains(startedState.log_tail[0]);

    cy.get('[data-cy=cutthroat-deck]').click();
    cy.window().then((win) => {
      const ws = getGameWs(win, gameId);
      const sent = JSON.parse(ws.sent[0]);
      expect(sent.action).to.deep.equal({ type: 'Draw' });

      ws.emit({
        type: 'state',
        ...startedState,
        version: startedState.version + 1,
        legal_actions: [],
      });
    });
  });

  it('mobile layout fits viewport without vertical page scrolling', () => {
    const gameId = 899;
    const startedState = buildStartedState({
      gameId,
      legalActions: [ { type: 'Draw' } ],
    });

    cy.viewport(390, 844);
    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/state`, startedState);
    cy.intercept('POST', `/cutthroat/api/v1/games/${gameId}/join`, { seat: 0 });

    cy.visit(`/cutthroat/game/${gameId}`, {
      onBeforeLoad(win) {
        installMockCutthroatWs(win);
      },
    });

    cy.document().then((doc) => {
      const scrollingEl = doc.scrollingElement;
      expect(scrollingEl.scrollHeight).to.be.lte(scrollingEl.clientHeight + 1);
    });
  });

  it('redirects non-player game route to spectate route and remains read-only', () => {
    const gameId = 810;
    const spectatorState = buildStartedState({
      gameId,
      isSpectator: true,
      legalActions: [ { type: 'Draw' } ],
    });

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/state`, spectatorState);
    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/spectate/state`, spectatorState);

    cy.visit(`/cutthroat/game/${gameId}`, {
      onBeforeLoad(win) {
        installMockCutthroatWs(win);
      },
    });

    cy.url().should('include', `/cutthroat/spectate/${gameId}`);
    cy.get('[data-cy=cutthroat-deck]').click();
    cy.window().then((win) => {
      const ws = getGameWs(win, gameId);
      expect(ws.sent).to.have.length(0);
    });
  });

  it('redirects self spectate route back to game route', () => {
    const gameId = 811;
    const playerState = buildStartedState({
      gameId,
      isSpectator: false,
      legalActions: [ { type: 'Draw' } ],
    });

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/spectate/state`, {
      statusCode: 409,
      body: { message: 'conflict' },
    });
    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/state`, playerState);

    cy.visit(`/cutthroat/spectate/${gameId}`, {
      onBeforeLoad(win) {
        installMockCutthroatWs(win);
      },
    });

    cy.url().should('include', `/cutthroat/game/${gameId}`);
  });

  it('hand -> move choice -> points sends PlayPoints', () => {
    const gameId = 802;
    const startedState = buildStartedState({
      gameId,
      legalActions: [
        { type: 'PlayPoints', data: { card: '3C' } },
      ],
    });

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/state`, startedState);
    cy.intercept('POST', `/cutthroat/api/v1/games/${gameId}/join`, { seat: 0 });

    cy.visit(`/cutthroat/game/${gameId}`, {
      onBeforeLoad(win) {
        installMockCutthroatWs(win);
      },
    });

    cy.get('[data-cutthroat-hand-card="3C"]').click();
    cy.get('[data-cy=cutthroat-move-choice-points]').click();

    cy.window().then((win) => {
      const ws = getGameWs(win, gameId);
      const sent = JSON.parse(ws.sent[0]);
      expect(sent.action).to.deep.equal({
        type: 'PlayPoints',
        data: {
          card: '3C',
        },
      });

      ws.emit({
        type: 'state',
        ...startedState,
        version: startedState.version + 1,
        legal_actions: [],
      });
    });
  });

  it('targeted scuttle flow sends the selected target action', () => {
    const gameId = 803;
    const startedState = buildStartedState({
      gameId,
      legalActions: [
        {
          type: 'Scuttle',
          data: {
            card: '6C',
            target_point_base: '5C',
          },
        },
      ],
      players: [
        {
          seat: 0,
          hand: [ known('6C') ],
          points: [],
          royals: [],
          frozen: [],
        },
        {
          seat: 1,
          hand: [ hidden() ],
          points: [ { base: '5C', controller: 1, jacks: [] } ],
          royals: [],
          frozen: [],
        },
        {
          seat: 2,
          hand: [ hidden() ],
          points: [],
          royals: [],
          frozen: [],
        },
      ],
    });

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/state`, startedState);
    cy.intercept('POST', `/cutthroat/api/v1/games/${gameId}/join`, { seat: 0 });

    cy.visit(`/cutthroat/game/${gameId}`, {
      onBeforeLoad(win) {
        installMockCutthroatWs(win);
      },
    });

    cy.get('[data-cutthroat-hand-card="6C"]').click();
    cy.get('[data-cy=cutthroat-move-choice-scuttle]').click();
    cy.get('[data-cutthroat-point-card="5C"]').click();

    cy.window().then((win) => {
      const ws = getGameWs(win, gameId);
      const sent = JSON.parse(ws.sent[0]);
      expect(sent.action).to.deep.equal({
        type: 'Scuttle',
        data: {
          card: '6C',
          target_point_base: '5C',
        },
      });

      ws.emit({
        type: 'state',
        ...startedState,
        version: startedState.version + 1,
        legal_actions: [],
      });
    });
  });

  it('countering phase supports CounterPass and CounterTwo interactions', () => {
    const gameId = 804;
    const counterState = buildStartedState({
      gameId,
      phase: {
        type: 'Countering',
        data: {
          next_seat: 0,
          twos: [],
        },
      },
      turn: 1,
      tokenlog: 'V1 CUTTHROAT3P DEALER P0 DECK AC AD AH AS ENDDECK P1 MT_ONEOFF 4C TGT_P P0',
      legalActions: [
        { type: 'CounterPass' },
        { type: 'CounterTwo', data: { two_card: '2C' } },
      ],
      players: [
        {
          seat: 0,
          hand: [ known('2C') ],
          points: [],
          royals: [],
          frozen: [],
        },
        {
          seat: 1,
          hand: [ hidden() ],
          points: [],
          royals: [],
          frozen: [],
        },
        {
          seat: 2,
          hand: [ hidden() ],
          points: [],
          royals: [],
          frozen: [],
        },
      ],
    });

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/state`, counterState);
    cy.intercept('POST', `/cutthroat/api/v1/games/${gameId}/join`, { seat: 0 });

    cy.visit(`/cutthroat/game/${gameId}`, {
      onBeforeLoad(win) {
        installMockCutthroatWs(win);
      },
    });

    cy.get('#counter-dialog').should('be.visible');
    cy.get('[data-cy=decline-counter-resolve]').click();
    cy.window().then((win) => {
      const ws = getGameWs(win, gameId);
      const firstSent = JSON.parse(ws.sent[0]);
      expect(firstSent.action).to.deep.equal({ type: 'CounterPass' });

      ws.emit({
        type: 'state',
        ...counterState,
        version: counterState.version + 1,
      });
    });

    cy.get('#counter-dialog [data-cy=counter]').click();
    cy.get('[data-counter-dialog-card="2-0"]').click();
    cy.window().then((win) => {
      const ws = getGameWs(win, gameId);
      const secondSent = JSON.parse(ws.sent[1]);
      expect(secondSent.action).to.deep.equal({
        type: 'CounterTwo',
        data: {
          two_card: '2C',
        },
      });

      ws.emit({
        type: 'state',
        ...counterState,
        version: counterState.version + 2,
        legal_actions: [],
      });
    });
  });

  it('countering phase without twos shows cannot-counter dialog', () => {
    const gameId = 809;
    const counterState = buildStartedState({
      gameId,
      phase: {
        type: 'Countering',
        data: {
          next_seat: 0,
          twos: [],
        },
      },
      turn: 1,
      tokenlog: 'V1 CUTTHROAT3P DEALER P0 DECK AC AD AH AS ENDDECK P1 MT_ONEOFF 4C',
      legalActions: [
        { type: 'CounterPass' },
      ],
      players: [
        {
          seat: 0,
          hand: [ known('3C') ],
          points: [],
          royals: [],
          frozen: [],
        },
        {
          seat: 1,
          hand: [ hidden() ],
          points: [],
          royals: [],
          frozen: [],
        },
        {
          seat: 2,
          hand: [ hidden() ],
          points: [],
          royals: [],
          frozen: [],
        },
      ],
    });

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/state`, counterState);
    cy.intercept('POST', `/cutthroat/api/v1/games/${gameId}/join`, { seat: 0 });

    cy.visit(`/cutthroat/game/${gameId}`, {
      onBeforeLoad(win) {
        installMockCutthroatWs(win);
      },
    });

    cy.get('#cannot-counter-dialog').should('be.visible');
    cy.get('#cannot-counter-dialog').should('not.contain', '? as a one-off');
    cy.get('#cannot-counter-dialog').should('not.contain', 'effects[undefined]');
    cy.get('[data-cy=cannot-counter-resolve]').click();
    cy.window().then((win) => {
      const ws = getGameWs(win, gameId);
      const sent = JSON.parse(ws.sent[0]);
      expect(sent.action).to.deep.equal({ type: 'CounterPass' });
    });
  });

  it('four one-off player targeting uses chooser dialog and transitions to counter dialog', () => {
    const gameId = 880;
    const oneOffState = buildStartedState({
      gameId,
      legalActions: [
        {
          type: 'PlayOneOff',
          data: {
            card: '4C',
            target: {
              type: 'Player',
              data: { seat: 1 },
            },
          },
        },
        {
          type: 'PlayOneOff',
          data: {
            card: '4C',
            target: {
              type: 'Player',
              data: { seat: 2 },
            },
          },
        },
      ],
      players: [
        {
          seat: 0,
          hand: [ known('4C') ],
          points: [],
          royals: [],
          frozen: [],
        },
        {
          seat: 1,
          hand: [ hidden() ],
          points: [],
          royals: [],
          frozen: [],
        },
        {
          seat: 2,
          hand: [ hidden() ],
          points: [],
          royals: [],
          frozen: [],
        },
      ],
    });

    const counterState = buildStartedState({
      gameId,
      version: 2,
      phase: {
        type: 'Countering',
        data: {
          next_seat: 0,
          twos: [],
        },
      },
      turn: 1,
      tokenlog: 'V1 CUTTHROAT3P DEALER P0 DECK AC AD AH AS ENDDECK P0 MT_ONEOFF 4C TGT_P P1',
      legalActions: [
        { type: 'CounterPass' },
        { type: 'CounterTwo', data: { two_card: '2C' } },
      ],
      players: [
        {
          seat: 0,
          hand: [ known('2C') ],
          points: [],
          royals: [],
          frozen: [],
        },
        {
          seat: 1,
          hand: [ hidden() ],
          points: [],
          royals: [],
          frozen: [],
        },
        {
          seat: 2,
          hand: [ hidden() ],
          points: [],
          royals: [],
          frozen: [],
        },
      ],
    });

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/state`, oneOffState);
    cy.intercept('POST', `/cutthroat/api/v1/games/${gameId}/join`, { seat: 0 });

    cy.visit(`/cutthroat/game/${gameId}`, {
      onBeforeLoad(win) {
        installMockCutthroatWs(win);
      },
    });

    cy.get('[data-cutthroat-hand-card="4C"]').click();
    cy.get('[data-cy=cutthroat-move-choice-oneOff]').click();
    cy.get('#cutthroat-four-player-target-dialog').should('be.visible');
    cy.get('[data-cy=cutthroat-four-target-player-1]').click();

    cy.window().then((win) => {
      const ws = getGameWs(win, gameId);
      const sent = JSON.parse(ws.sent[0]);
      expect(sent.action).to.deep.equal({
        type: 'PlayOneOff',
        data: {
          card: '4C',
          target: {
            type: 'Player',
            data: { seat: 1 },
          },
        },
      });

      ws.emit({
        type: 'state',
        ...counterState,
      });
    });

    cy.get('#counter-dialog').should('be.visible');
  });

  it('resolving phases are clickable (three, four, and seven)', () => {
    const gameId = 805;
    const resolvingThreeState = buildStartedState({
      gameId,
      phase: {
        type: 'ResolvingThree',
        data: {
          seat: 0,
        },
      },
      legalActions: [
        { type: 'ResolveThreePick', data: { card_from_scrap: '3C' } },
        { type: 'ResolveThreePick', data: { card_from_scrap: '4D' } },
      ],
      scrap: [ '3C', '4D' ],
    });

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/state`, resolvingThreeState);
    cy.intercept('POST', `/cutthroat/api/v1/games/${gameId}/join`, { seat: 0 });

    cy.visit(`/cutthroat/game/${gameId}`, {
      onBeforeLoad(win) {
        installMockCutthroatWs(win);
      },
    });

    cy.get('[data-cy=cutthroat-scrap]').click();
    cy.get('#cutthroat-scrap-dialog').should('be.visible');
    cy.get('#cutthroat-scrap-dialog [data-scrap-dialog-card="4-1"]').click();
    cy.get('body').then(($body) => {
      if ($body.find('[data-cy=close-cutthroat-scrap-dialog-button]').length > 0) {
        cy.get('[data-cy=close-cutthroat-scrap-dialog-button]').click({ force: true });
      }
    });
    cy.get('#cutthroat-scrap-dialog').should('not.exist');
    cy.window().then((win) => {
      const ws = getGameWs(win, gameId);
      const firstSent = JSON.parse(ws.sent[0]);
      expect(firstSent.action).to.deep.equal({
        type: 'ResolveThreePick',
        data: {
          card_from_scrap: '4D',
        },
      });

      const resolvingFourState = buildStartedState({
        gameId,
        version: 2,
        phase: {
          type: 'ResolvingFour',
          data: {
            seat: 0,
            remaining: 1,
          },
        },
        legalActions: [
          { type: 'ResolveFourDiscard', data: { card: '7C' } },
        ],
        players: [
          {
            seat: 0,
            hand: [ known('7C') ],
            points: [],
            royals: [],
            frozen: [],
          },
          {
            seat: 1,
            hand: [ hidden() ],
            points: [],
            royals: [],
            frozen: [],
          },
          {
            seat: 2,
            hand: [ hidden() ],
            points: [],
            royals: [],
            frozen: [],
          },
        ],
      });

      ws.emit({
        type: 'state',
        ...resolvingFourState,
      });
    });

    cy.get('#cutthroat-four-discard-dialog').should('be.visible');
    cy.get('[data-discard-card="7C"]').click();
    cy.get('[data-cy=submit-four-dialog]').click();
    cy.window().then((win) => {
      const ws = getGameWs(win, gameId);
      const secondSent = JSON.parse(ws.sent[1]);
      expect(secondSent.action).to.deep.equal({
        type: 'ResolveFourDiscard',
        data: {
          card: '7C',
        },
      });

      const resolvingSevenState = buildStartedState({
        gameId,
        version: 3,
        phase: {
          type: 'ResolvingSeven',
          data: {
            seat: 0,
            revealed: 2,
            revealed_cards: [ '7C', 'KC' ],
          },
        },
        legalActions: [
          {
            type: 'ResolveSevenChoose',
            data: {
              source_index: 0,
              play: { type: 'Points' },
            },
          },
          {
            type: 'ResolveSevenChoose',
            data: {
              source_index: 1,
              play: { type: 'Royal' },
            },
          },
        ],
      });

      ws.emit({
        type: 'state',
        ...resolvingSevenState,
      });
    });

    cy.get('[data-cy=cutthroat-reveal-0]').click();
    cy.get('[data-cy=cutthroat-move-choice-points]').click();

    cy.window().then((win) => {
      const ws = getGameWs(win, gameId);
      const thirdSent = JSON.parse(ws.sent[2]);
      expect(thirdSent.action).to.deep.equal({
        type: 'ResolveSevenChoose',
        data: {
          source_index: 0,
          play: {
            type: 'Points',
          },
        },
      });
    });
  });

  it('renders scrap cards in dialog from object-shaped scrap payload and allows resolve-three pick', () => {
    const gameId = 906;
    const resolvingThreeState = buildStartedState({
      gameId,
      phase: {
        type: 'ResolvingThree',
        data: {
          seat: 0,
        },
      },
      legalActions: [
        { type: 'ResolveThreePick', data: { card_from_scrap: '3C' } },
        { type: 'ResolveThreePick', data: { card_from_scrap: 'TD' } },
        { type: 'ResolveThreePick', data: { card_from_scrap: 'J1' } },
      ],
      scrap: {
        0: { Standard: { rank: 'THREE', suit: 'CLUBS' } },
        1: { Standard: { rank: 'TEN', suit: 'DIAMONDS' } },
        2: { Joker: 1 },
      },
    });

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/state`, resolvingThreeState);
    cy.intercept('POST', `/cutthroat/api/v1/games/${gameId}/join`, { seat: 0 });

    cy.visit(`/cutthroat/game/${gameId}`, {
      onBeforeLoad(win) {
        installMockCutthroatWs(win);
      },
    });

    cy.get('[data-cy=cutthroat-scrap]').click();
    cy.get('#cutthroat-scrap-dialog').should('be.visible');
    cy.get('#cutthroat-scrap-dialog [data-scrap-dialog-card]:visible').should('have.length', 3);
    cy.contains('#cutthroat-scrap-dialog', 'There are no cards in the scrap pile.').should('not.exist');

    // ten of diamonds (rank 10, suit 1)
    cy.get('#cutthroat-scrap-dialog [data-scrap-dialog-card="10-1"]').click();

    cy.window().then((win) => {
      const ws = getGameWs(win, gameId);
      const sent = JSON.parse(ws.sent[0]);
      expect(sent.action).to.deep.equal({
        type: 'ResolveThreePick',
        data: {
          card_from_scrap: 'TD',
        },
      });
    });
  });

  it('long-presses scrap to send scrap_straighten and apply sync updates', () => {
    const gameId = 889;
    const startedState = buildStartedState({
      gameId,
      legalActions: [ { type: 'Draw' } ],
      scrap: [ '3C', '4D' ],
    });

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/state`, startedState);
    cy.intercept('POST', `/cutthroat/api/v1/games/${gameId}/join`, { seat: 0 });

    cy.visit(`/cutthroat/game/${gameId}`, {
      onBeforeLoad(win) {
        installMockCutthroatWs(win);
      },
    });

    cy.get('[data-cy=cutthroat-scrap]').trigger('mousedown');
    cy.wait(650);
    cy.get('[data-cy=cutthroat-scrap]').trigger('mouseup', { force: true });

    cy.window().then((win) => {
      const ws = getGameWs(win, gameId);
      const sent = JSON.parse(ws.sent[0]);
      expect(sent).to.deep.equal({ type: 'scrap_straighten' });

      ws.emit({
        type: 'scrap_straighten',
        game_id: gameId,
        straightened: true,
        actor_seat: 0,
      });
    });

    cy.get('[data-cy=cutthroat-scrap]').should('have.class', 'is-straightened');

    cy.window().then((win) => {
      const ws = getGameWs(win, gameId);
      ws.emit({
        type: 'scrap_straighten',
        game_id: gameId,
        straightened: false,
        actor_seat: 1,
      });
    });

    cy.get('[data-cy=cutthroat-scrap]').should('not.have.class', 'is-straightened');
  });

  it('shows snackbar feedback when websocket action fails', () => {
    const gameId = 888;
    const startedState = buildStartedState({
      gameId,
      legalActions: [ { type: 'Draw' } ],
    });

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/state`, startedState);
    cy.intercept('POST', `/cutthroat/api/v1/games/${gameId}/join`, { seat: 0 });

    cy.visit(`/cutthroat/game/${gameId}`, {
      onBeforeLoad(win) {
        installMockCutthroatWs(win);
      },
    });

    cy.get('[data-cy=cutthroat-deck]').click();
    cy.window().then((win) => {
      const ws = getGameWs(win, gameId);
      ws.emit({
        type: 'error',
        code: 400,
        message: 'illegal action',
      });
    });

    cy.get('[data-cy=global-snackbar]')
      .should('exist')
      .and('contain', 'illegal action');
  });

  it('shows royal glasses eight in left opponent zone for viewer seat 0', () => {
    const gameId = 890;
    const startedState = buildStartedState({
      gameId,
      seat: 0,
      turn: 0,
      legalActions: [],
      players: buildRoyalEightPlayers(0),
    });

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/state`, startedState);
    cy.intercept('POST', `/cutthroat/api/v1/games/${gameId}/join`, { seat: 0 });

    cy.visit(`/cutthroat/game/${gameId}`, {
      onBeforeLoad(win) {
        installMockCutthroatWs(win);
      },
    });

    cy.get('[data-cutthroat-royal-card="8C"]').should('have.length', 1);
    cy.get('.player-area.float-left [data-cutthroat-royal-card="8C"]').should('exist');
    cy.get('.player-area.float-left [data-cutthroat-royal-card="8C"] .player-card').should('have.class', 'glasses');
  });

  it('shows royal glasses eight in self zone for viewer seat 1', () => {
    const gameId = 891;
    const startedState = buildStartedState({
      gameId,
      seat: 1,
      turn: 1,
      legalActions: [],
      players: buildRoyalEightPlayers(1),
    });

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/state`, startedState);
    cy.intercept('POST', `/cutthroat/api/v1/games/${gameId}/join`, { seat: 1 });

    cy.visit(`/cutthroat/game/${gameId}`, {
      onBeforeLoad(win) {
        installMockCutthroatWs(win);
      },
    });

    cy.get('[data-cutthroat-royal-card="8C"]').should('have.length', 1);
    cy.get('.player-area.me [data-cutthroat-royal-card="8C"]').should('exist');
    cy.get('.player-area.me [data-cutthroat-royal-card="8C"] .player-card').should('have.class', 'glasses');
  });

  it('shows royal glasses eight in right opponent zone for viewer seat 2', () => {
    const gameId = 892;
    const startedState = buildStartedState({
      gameId,
      seat: 2,
      turn: 2,
      legalActions: [],
      players: buildRoyalEightPlayers(2),
    });

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/state`, startedState);
    cy.intercept('POST', `/cutthroat/api/v1/games/${gameId}/join`, { seat: 2 });

    cy.visit(`/cutthroat/game/${gameId}`, {
      onBeforeLoad(win) {
        installMockCutthroatWs(win);
      },
    });

    cy.get('[data-cutthroat-royal-card="8C"]').should('have.length', 1);
    cy.get('.player-area.float-right [data-cutthroat-royal-card="8C"]').should('exist');
    cy.get('.player-area.float-right [data-cutthroat-royal-card="8C"] .player-card').should('have.class', 'glasses');
  });
});
