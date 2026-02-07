import { setupCutthroatUser } from './setup';

Cypress.Commands.add('setupCutthroatUser', () => {
  return setupCutthroatUser();
});

Cypress.Commands.add('seedCutthroatGameFromTokenlog', ({
  gameId,
  tokenlog,
  dealerSeat,
  status,
  name,
  players,
  playerSeat = 0,
  spectatingUsernames,
}) => {
  if (!gameId || !tokenlog) {
    throw new Error('seedCutthroatGameFromTokenlog requires gameId and tokenlog');
  }

  return cy.get('@cutthroatUser').then((user) => {
    const defaultPlayers = [
      {
        seat: playerSeat,
        user_id: user.id,
        username: user.username,
        ready: true,
      },
      {
        seat: (playerSeat + 1) % 3,
        user_id: 90001,
        username: 'cutthroat-opponent-1',
        ready: true,
      },
      {
        seat: (playerSeat + 2) % 3,
        user_id: 90002,
        username: 'cutthroat-opponent-2',
        ready: true,
      },
    ];

    const body = {
      game_id: gameId,
      players: players ?? defaultPlayers,
      tokenlog,
      dealer_seat: dealerSeat,
      status,
      spectating_usernames: spectatingUsernames,
      name,
    };

    return cy
      .request('POST', '/cutthroat/api/test/games/seed-tokenlog', body)
      .its('body')
      .then((response) => {
        expect(response.game_id).to.eq(gameId);
        expect(response.tokenlog).to.eq(tokenlog);
        return response;
      });
  });
});

Cypress.Commands.add('openCutthroatGame', (gameId, route = 'game') => {
  cy.visit(`/cutthroat/${route}/${gameId}`);
  cy.get('#cutthroat-game-wrapper').should('be.visible');
});
