import { transcriptWithActions } from '../../../support/cutthroat/seed';
import { CUTTHROAT_SELECTORS } from '../../../support/cutthroat/selectors';

describe('Cutthroat 3P Spectating', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('When a viewer opens a started 3P game in spectate mode and attempts gameplay interaction, then the server state version does not change because spectator mode must be strictly read-only.', () => {
    const gameId = 7331;
    const transcript = transcriptWithActions({ dealer: 'P2' });

    cy.seedCutthroatGameFromTranscript({
      gameId,
      ...transcript,
      status: 1,
      players: [
        { seat: 0, user_id: 92001, username: 's0', ready: true },
        { seat: 1, user_id: 92002, username: 's1', ready: true },
        { seat: 2, user_id: 92003, username: 's2', ready: true },
      ],
      spectatingUsernames: [ 'spec-a', 'spec-b' ],
    });

    cy.openCutthroatGame(gameId, 'spectate');
    cy.get(CUTTHROAT_SELECTORS.seatStatuses).should('have.length', 3);
    cy.get(CUTTHROAT_SELECTORS.seatPoints).each(($el) => {
      cy.wrap($el).should('contain', 'POINTS');
    });
    cy.get(CUTTHROAT_SELECTORS.seatGoals).each(($el) => {
      cy.wrap($el).should('contain', 'GOAL');
    });
    cy.get(`${CUTTHROAT_SELECTORS.turnIndicator}.my-turn`).should('have.length', 1);
    cy.get(CUTTHROAT_SELECTORS.spectatorListButton)
      .should('contain', '2')
      .click();
    cy.get(CUTTHROAT_SELECTORS.spectatorListMenu)
      .should('contain', 'Spectators')
      .should('contain', 'spec-a')
      .should('contain', 'spec-b');
    cy.contains('p', 'Spectators:').should('not.exist');
    cy.get(CUTTHROAT_SELECTORS.deck).click();

    cy.request(`/cutthroat/api/v1/games/${gameId}/spectate/state`)
      .its('body')
      .then((before) => {
        cy.request(`/cutthroat/api/v1/games/${gameId}/spectate/state`)
          .its('body')
          .then((after) => {
            expect(after.version).to.eq(before.version);
          });
      });
  });
});
