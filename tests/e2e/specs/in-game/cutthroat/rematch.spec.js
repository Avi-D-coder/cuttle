import { transcriptWithActions } from '../../../support/cutthroat/seed';

describe('Cutthroat 3P Rematch UX', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('keeps player on game route and allows canceling rematch offer', () => {
    const gameId = 7601;
    const transcript = transcriptWithActions({ dealer: 'P2' });

    cy.seedCutthroatGameFromTranscript({
      gameId,
      ...transcript,
      status: 2,
      playerSeat: 0,
    });

    cy.intercept('POST', `/cutthroat/api/v1/games/${gameId}/rematch`).as('rematchRequest');
    cy.intercept('POST', '/cutthroat/api/v1/games/*/ready').as('readyRequest');

    cy.openCutthroatGame(gameId, 'game');
    cy.location('pathname').should('eq', `/cutthroat/game/${gameId}`);

    cy.get('[data-cy=cutthroat-rematch-btn]').should('contain', 'Rematch')
      .click();

    cy.wait('@rematchRequest')
      .its('response.body.id')
      .should('be.a', 'number')
      .as('rematchGameId');

    cy.wait('@readyRequest')
      .then(({ request }) => {
        expect(request.body).to.deep.eq({ ready: true });
      });

    cy.location('pathname').should('eq', `/cutthroat/game/${gameId}`);
    cy.get('[data-cy=cutthroat-rematch-waiting]').should('be.visible');
    cy.get('[data-cy=cutthroat-rematch-btn]').should('contain', 'Unready');

    cy.get('[data-cy=cutthroat-rematch-btn]').then(($btn) => {
      $btn[0].click();
    });
    cy.wait('@readyRequest')
      .then(({ request }) => {
        expect(request.body).to.deep.eq({ ready: false });
      });

    cy.location('pathname').should('eq', `/cutthroat/game/${gameId}`);
    cy.get('[data-cy=cutthroat-rematch-waiting]').should('not.exist');
    cy.get('[data-cy=cutthroat-rematch-btn]').should('contain', 'Rematch');
  });

  it('allows rejoining reserved rematch lobby from home after leaving old game page', () => {
    const gameId = 7602;
    const transcript = transcriptWithActions({ dealer: 'P2' });

    cy.seedCutthroatGameFromTranscript({
      gameId,
      ...transcript,
      status: 2,
      playerSeat: 0,
    });

    cy.intercept('POST', `/cutthroat/api/v1/games/${gameId}/rematch`).as('rematchRequest');
    cy.intercept('POST', '/cutthroat/api/v1/games/*/ready').as('readyRequest');

    cy.openCutthroatGame(gameId, 'game');
    cy.get('[data-cy=cutthroat-rematch-btn]').click();

    cy.wait('@readyRequest')
      .then(({ request }) => {
        expect(request.body).to.deep.eq({ ready: true });
      });
    cy.wait('@rematchRequest')
      .its('response.body.id')
      .should('be.a', 'number')
      .as('rematchGameId');

    cy.visit('/');
    cy.location('pathname').should('eq', '/');

    cy.get('@rematchGameId').then((id) => {
      cy.get(`[data-cy=cutthroat-join-lobby-${id}]`)
        .should('be.visible')
        .and('be.enabled');
      cy.get(`[data-cy=cutthroat-join-lobby-${id}]`)
        .click();
      cy.location('pathname').should('eq', `/cutthroat/lobby/${id}`);
    });
  });

  it('allows spectators to opt into following the next rematch game', () => {
    const gameId = 7603;
    const nextGameId = 8701;
    const transcript = transcriptWithActions({ dealer: 'P2' });
    const spectatorPlayers = [
      { seat: 0, user_id: 93001, username: 's0', ready: true },
      { seat: 1, user_id: 93002, username: 's1', ready: true },
      { seat: 2, user_id: 93003, username: 's2', ready: true },
    ];

    cy.seedCutthroatGameFromTranscript({
      gameId,
      ...transcript,
      status: 2,
      players: spectatorPlayers,
    });
    cy.seedCutthroatGameFromTranscript({
      gameId: nextGameId,
      ...transcript,
      status: 1,
      players: spectatorPlayers,
    });

    cy.visit(`/cutthroat/spectate/${gameId}?gameStateIndex=0`);
    cy.get('#cutthroat-game-wrapper').should('be.visible');
    cy.location('search').should('include', 'gameStateIndex=0');
    cy.window()
      .its('cuttle.cutthroatStore')
      .then((store) => {
        store.disconnectWs();
        store.hasActiveSeatedPlayers = true;
        store.spectateGames = [];
      });
    cy.get('[data-cy=cutthroat-rematch-btn]').should('contain', 'Spectate');
    cy.get('[data-cy=cutthroat-rematch-btn]').then(($btn) => {
      $btn[0].click();
    });

    cy.window()
      .its('cuttle.cutthroatStore')
      .then((store) => {
        store.disconnectLobbyWs();
        store.spectateGames = [
          {
            id: nextGameId,
            name: 'rematch',
            seat_count: 3,
            status: 1,
            rematch_from_game_id: gameId,
            spectating_usernames: [],
          },
        ];
      });

    cy.location('pathname').should('eq', `/cutthroat/spectate/${nextGameId}`);
    cy.location('search').should('include', 'gameStateIndex=-1');
    cy.window()
      .its('cuttle.cutthroatStore.gameId')
      .should('eq', nextGameId);
  });

  it('cancels spectator follow when leaving replay end before rematch appears', () => {
    const gameId = 7604;
    const nextGameId = 8702;
    const transcript = transcriptWithActions({
      dealer: 'P2',
      actions: [ 'P0 draw' ],
    });
    const spectatorPlayers = [
      { seat: 0, user_id: 93101, username: 'r0', ready: true },
      { seat: 1, user_id: 93102, username: 'r1', ready: true },
      { seat: 2, user_id: 93103, username: 'r2', ready: true },
    ];
    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/spectate/state?gameStateIndex=-1`).as('spectateReplayEnd');

    cy.seedCutthroatGameFromTranscript({
      gameId,
      ...transcript,
      status: 2,
      players: spectatorPlayers,
    });
    cy.seedCutthroatGameFromTranscript({
      gameId: nextGameId,
      ...transcript,
      status: 1,
      players: spectatorPlayers,
    });

    cy.visit(`/cutthroat/spectate/${gameId}?gameStateIndex=0`);
    cy.get('#cutthroat-game-wrapper').should('be.visible');
    cy.location('search').should('include', 'gameStateIndex=0');
    cy.get('[data-cy=skip-forward]').then(($btn) => {
      $btn[0].click();
    });
    cy.location('search').should('include', 'gameStateIndex=-1');
    cy.wait('@spectateReplayEnd');
    cy.window()
      .its('cuttle.cutthroatStore')
      .then((store) => {
        store.disconnectWs();
        store.hasActiveSeatedPlayers = true;
      });

    cy.get('[data-cy=cutthroat-rematch-btn]').then(($btn) => {
      $btn[0].click();
    });
    cy.get('[data-cy=cutthroat-rematch-waiting]').should('be.visible');

    cy.get('[data-cy=step-backward]').click();
    cy.location('search').should('include', 'gameStateIndex=0');
    cy.get('[data-cy=cutthroat-rematch-waiting]').should('not.exist');

    cy.window()
      .its('cuttle.cutthroatStore')
      .then((store) => {
        store.spectateGames = [
          {
            id: nextGameId,
            name: 'rematch',
            seat_count: 3,
            status: 1,
            rematch_from_game_id: gameId,
            spectating_usernames: [],
          },
        ];
      });

    cy.wait(200);
    cy.location('pathname').should('eq', `/cutthroat/spectate/${gameId}`);
  });

  it('hides spectator follow button when no rematch signal exists', () => {
    const gameId = 7607;
    const transcript = transcriptWithActions({ dealer: 'P2' });
    const spectatorPlayers = [
      { seat: 0, user_id: 93401, username: 'z0', ready: true },
      { seat: 1, user_id: 93402, username: 'z1', ready: true },
      { seat: 2, user_id: 93403, username: 'z2', ready: true },
    ];

    cy.seedCutthroatGameFromTranscript({
      gameId,
      ...transcript,
      status: 2,
      players: spectatorPlayers,
    });

    cy.visit(`/cutthroat/spectate/${gameId}?gameStateIndex=0`);
    cy.get('#cutthroat-game-wrapper').should('be.visible');
    cy.location('search').should('include', 'gameStateIndex=0');
    cy.get('[data-cy=cutthroat-rematch-btn]').should('not.exist');
    cy.get('[data-cy=cutthroat-replay-next-game-btn]').should('not.exist');
  });

  it('shows replay-next-game button when next game replay is available', () => {
    const gameId = 7605;
    const nextGameId = 8703;
    const transcript = transcriptWithActions({
      dealer: 'P2',
      actions: [],
    });
    const spectatorPlayers = [
      { seat: 0, user_id: 93201, username: 'n0', ready: true },
      { seat: 1, user_id: 93202, username: 'n1', ready: true },
      { seat: 2, user_id: 93203, username: 'n2', ready: true },
    ];

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/spectate/state?gameStateIndex=0`).as('spectateReplayStart');

    cy.seedCutthroatGameFromTranscript({
      gameId,
      ...transcript,
      status: 2,
      players: spectatorPlayers,
    });
    cy.seedCutthroatGameFromTranscript({
      gameId: nextGameId,
      ...transcript,
      status: 2,
      players: spectatorPlayers,
    });

    cy.visit(`/cutthroat/spectate/${gameId}?gameStateIndex=0`);
    cy.get('#cutthroat-game-wrapper').should('be.visible');
    cy.location('search').should('include', 'gameStateIndex=0');
    cy.wait('@spectateReplayStart');
    cy.window()
      .its('cuttle.cutthroatStore')
      .then((store) => {
        store.disconnectWs();
        store.nextGameId = nextGameId;
        store.nextGameFinished = true;
      });
    cy.window().its('cuttle.cutthroatStore.nextGameId')
      .should('eq', nextGameId);
    cy.window().its('cuttle.cutthroatStore.nextGameFinished')
      .should('eq', true);

    cy.get('[data-cy=cutthroat-replay-next-game-btn]', { timeout: 10000 })
      .should('be.visible');
    cy.get('[data-cy=cutthroat-rematch-btn]').should('not.exist');
    cy.get('[data-cy=cutthroat-replay-next-game-btn]').click();
    cy.location('pathname').should('eq', `/cutthroat/spectate/${nextGameId}`);
    cy.location('search').should('include', 'gameStateIndex=0');
    cy.window()
      .its('cuttle.cutthroatStore.gameId')
      .should('eq', nextGameId);
  });

  it('keeps finished-game deep links replayable after returning home', () => {
    const gameId = 7606;
    const transcript = transcriptWithActions({
      dealer: 'P2',
      actions: [ 'P0 draw' ],
    });
    const spectatorPlayers = [
      { seat: 0, user_id: 93301, username: 'd0', ready: true },
      { seat: 1, user_id: 93302, username: 'd1', ready: true },
      { seat: 2, user_id: 93303, username: 'd2', ready: true },
    ];

    cy.seedCutthroatGameFromTranscript({
      gameId,
      ...transcript,
      status: 2,
      players: spectatorPlayers,
    });

    cy.visit(`/cutthroat/spectate/${gameId}`);
    cy.get('#cutthroat-game-wrapper').should('be.visible');
    cy.location('pathname').should('eq', `/cutthroat/spectate/${gameId}`);
    cy.location('search').should('include', 'gameStateIndex=0');

    cy.visit('/');
    cy.location('pathname').should('eq', '/');

    cy.wait(6000);

    cy.visit(`/cutthroat/spectate/${gameId}`);
    cy.get('#cutthroat-game-wrapper').should('be.visible');
    cy.location('pathname').should('eq', `/cutthroat/spectate/${gameId}`);
    cy.location('search').should('include', 'gameStateIndex=0');

    cy.visit(`/cutthroat/game/${gameId}`);
    cy.get('#cutthroat-game-wrapper').should('be.visible');
    cy.location('pathname').should('eq', `/cutthroat/spectate/${gameId}`);
    cy.location('search').should('include', 'gameStateIndex=0');
    cy.contains('Failed to join game').should('not.exist');
  });
});
