import { transcriptWithActions } from '../../../support/cutthroat/seed';

function buildSpectatorPlayers(prefix, baseUserId) {
  return [
    { seat: 0, user_id: baseUserId, username: `${prefix}0`, ready: true },
    { seat: 1, user_id: baseUserId + 1, username: `${prefix}1`, ready: true },
    { seat: 2, user_id: baseUserId + 2, username: `${prefix}2`, ready: true },
  ];
}

describe('Cutthroat 3P Rematch UX', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('When a finished-game player requests a rematch and then cancels readiness, then the user stays on the original game route and waiting state clears because rematch intent should be reversible without route churn.', () => {
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

    cy.wait('@rematchRequest').its('response.body.id')
      .should('be.a', 'number');
    cy.wait('@readyRequest').then(({ request }) => {
      expect(request.body).to.deep.eq({ ready: true });
    });

    cy.location('pathname').should('eq', `/cutthroat/game/${gameId}`);
    cy.get('[data-cy=cutthroat-rematch-waiting]')
      .should('be.visible')
      .and('contain', 'Waiting for Players')
      .and('contain', 'cutthroat-opponent-1')
      .and('contain', 'cutthroat-opponent-2');
    cy.get('[data-cy=cutthroat-rematch-btn]').should('contain', 'Unready')
      .click();

    cy.wait('@readyRequest').then(({ request }) => {
      expect(request.body).to.deep.eq({ ready: false });
    });

    cy.location('pathname').should('eq', `/cutthroat/game/${gameId}`);
    cy.get('[data-cy=cutthroat-rematch-waiting]').should('not.exist');
    cy.get('[data-cy=cutthroat-rematch-btn]').should('contain', 'Rematch');
  });

  it('When a player creates a rematch lobby and leaves to home, then the reserved rematch lobby remains directly rejoinable because post-game regrouping must survive navigation away from the old game page.', () => {
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

    cy.wait('@readyRequest').then(({ request }) => {
      expect(request.body).to.deep.eq({ ready: true });
    });
    cy.wait('@rematchRequest').its('response.body.id')
      .should('be.a', 'number')
      .as('rematchGameId');

    cy.visit('/');
    cy.location('pathname').should('eq', '/');

    cy.get('@rematchGameId').then((id) => {
      cy.visit(`/cutthroat/lobby/${id}`);
      cy.location('pathname').should('eq', `/cutthroat/lobby/${id}`);
    });
  });

  it('When a full rematch lobby includes the viewer as a reserved seat, then the join control remains enabled from the Cutthroat list because reserved rematch seats must stay reclaimable even at full seat count.', () => {
    const sourceGameId = 7605;
    const transcript = transcriptWithActions({ dealer: 'P2' });

    cy.seedCutthroatGameFromTranscript({
      gameId: sourceGameId,
      ...transcript,
      status: 2,
      playerSeat: 0,
    });
    cy.request('POST', `/cutthroat/api/v1/games/${sourceGameId}/rematch`)
      .its('body.id')
      .should('be.a', 'number')
      .as('rematchLobbyId');

    cy.visit('/cutthroat');
    cy.get('@rematchLobbyId').then((rematchLobbyId) => {
      cy.window()
        .its('cuttle.cutthroatStore')
        .then((store) => store.connectLobbyWs({ replace: true }));
      cy.window()
        .its('cuttle.cutthroatStore.lobbies', { timeout: 20000 })
        .should((lobbies) => {
          expect(lobbies.some((entry) => entry.id === rematchLobbyId)).to.eq(true);
        });
      cy.get(`[data-cy=cutthroat-join-lobby-${rematchLobbyId}]`, { timeout: 20000 })
        .closest('[data-cy=cutthroat-list-item]')
        .should('contain.text', '0 / 3 players')
        .and('not.contain.text', '3 / 3 players');
      cy.get(`[data-cy=cutthroat-join-lobby-${rematchLobbyId}]`, { timeout: 20000 })
        .should('be.visible')
        .and('be.enabled')
        .click();
      cy.location('pathname').should('eq', `/cutthroat/lobby/${rematchLobbyId}`);
    });
  });

  it('When a spectator opts into following rematches at replay end and a linked next game is already spectatable, then the client auto-navigates to that game because follow mode is an explicit continuity request.', () => {
    const gameId = 7603;
    const nextGameId = 8701;
    const transcript = transcriptWithActions({ dealer: 'P2' });
    const spectatorPlayers = buildSpectatorPlayers('s', 93001);

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
      rematchFromGameId: gameId,
    });

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/spectate/state?gameStateIndex=-1`).as('spectateReplayEnd');

    cy.visit(`/cutthroat/spectate/${gameId}?gameStateIndex=-1`);
    cy.get('#cutthroat-game-wrapper').should('be.visible');
    cy.location('search').should('include', 'gameStateIndex=-1');
    cy.wait('@spectateReplayEnd');

    cy.window()
      .its('cuttle.cutthroatStore')
      .then((store) => {
        store.disconnectWs();
        store.hasActiveSeatedPlayers = true;
        store.spectateGames = [];
      });
    cy.get('[data-cy=cutthroat-rematch-btn]').should('contain', 'Spectate')
      .then(($btn) => {
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

    cy.location('pathname', { timeout: 10000 }).should('eq', `/cutthroat/spectate/${nextGameId}`);
    cy.location('search').should('include', 'gameStateIndex=-1');
  });

  it('When a spectator starts follow mode at replay end but then steps backward before the rematch becomes spectatable, then follow is canceled and later rematch publication does not auto-redirect because replay context change should revoke pending follow intent.', () => {
    const gameId = 7604;
    const nextGameId = 8702;
    const transcript = transcriptWithActions({
      dealer: 'P2',
      actions: [ 'P0 draw' ],
    });
    const spectatorPlayers = buildSpectatorPlayers('r', 93101);

    cy.seedCutthroatGameFromTranscript({
      gameId,
      ...transcript,
      status: 2,
      players: spectatorPlayers,
    });
    cy.seedCutthroatGameFromTranscript({
      gameId: nextGameId,
      ...transcript,
      status: 0,
      players: spectatorPlayers,
      isRematchLobby: true,
      rematchFromGameId: gameId,
    });

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/spectate/state?gameStateIndex=-1`).as('spectateReplayEnd');

    cy.visit(`/cutthroat/spectate/${gameId}?gameStateIndex=-1`);
    cy.get('#cutthroat-game-wrapper').should('be.visible');
    cy.location('search').should('include', 'gameStateIndex=-1');
    cy.wait('@spectateReplayEnd');

    cy.window()
      .its('cuttle.cutthroatStore')
      .then((store) => {
        store.disconnectWs();
        store.hasActiveSeatedPlayers = true;
      });
    cy.get('[data-cy=cutthroat-rematch-btn]').should('contain', 'Spectate')
      .then(($btn) => {
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

    cy.location('pathname', { timeout: 8000 }).should('eq', `/cutthroat/spectate/${gameId}`);
  });

  it('When a finished replay has no rematch linkage or active-seated signal, then no spectator follow or replay-next controls are shown because rematch affordances should only appear when there is concrete continuation metadata.', () => {
    const gameId = 7607;
    const transcript = transcriptWithActions({ dealer: 'P2' });
    const spectatorPlayers = buildSpectatorPlayers('z', 93401);

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

  it('When a finished replay includes a linked and finished next game, then Replay Next Game appears and opens replay-start for that game because series replay navigation must remain deterministic.', () => {
    const gameId = 7608;
    const nextGameId = 8703;
    const transcript = transcriptWithActions({
      dealer: 'P2',
      actions: [],
    });
    const spectatorPlayers = buildSpectatorPlayers('n', 93201);

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
      rematchFromGameId: gameId,
    });

    cy.intercept('GET', `/cutthroat/api/v1/games/${gameId}/spectate/state?gameStateIndex=0`).as('spectateReplayStart');

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

    cy.get('[data-cy=cutthroat-replay-next-game-btn]', { timeout: 10000 })
      .should('be.visible')
      .click();
    cy.get('[data-cy=cutthroat-rematch-btn]').should('not.exist');
    cy.location('pathname').should('eq', `/cutthroat/spectate/${nextGameId}`);
    cy.location('search').should('include', 'gameStateIndex=0');
  });

  it('When a user revisits a finished-game deep link after returning home, then the link remains replayable and /game redirects to spectate because finished games should never regress into join-flow errors.', () => {
    const gameId = 7606;
    const transcript = transcriptWithActions({
      dealer: 'P2',
      actions: [ 'P0 draw' ],
    });
    const spectatorPlayers = buildSpectatorPlayers('d', 93301);

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
