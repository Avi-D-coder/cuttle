import { tokenlogWithActions } from '../../../support/cutthroat/seed';

describe('Cutthroat 3P Rematch UX', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('keeps player on game route and allows canceling rematch offer', () => {
    const gameId = 7601;
    const tokenlog = tokenlogWithActions({ dealer: 'P2' });

    cy.seedCutthroatGameFromTokenlog({
      gameId,
      tokenlog,
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

    cy.get('[data-cy=cutthroat-rematch-btn]').click();
    cy.wait('@readyRequest')
      .then(({ request }) => {
        expect(request.body).to.deep.eq({ ready: false });
      });

    cy.location('pathname').should('eq', `/cutthroat/game/${gameId}`);
    cy.get('[data-cy=cutthroat-rematch-waiting]').should('not.exist');
    cy.get('[data-cy=cutthroat-rematch-btn]').should('contain', 'Rematch');
  });
});
