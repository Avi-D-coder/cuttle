import { tokenlogWithActions } from '../../../support/cutthroat/seed';
import { CUTTHROAT_SELECTORS } from '../../../support/cutthroat/selectors';

describe('Cutthroat 3P Spectating', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('spectator mode remains read-only', () => {
    const gameId = 7331;
    const tokenlog = tokenlogWithActions({ dealer: 'P2' });

    cy.seedCutthroatGameFromTokenlog({
      gameId,
      tokenlog,
      status: 1,
      players: [
        { seat: 0, user_id: 92001, username: 's0', ready: true },
        { seat: 1, user_id: 92002, username: 's1', ready: true },
        { seat: 2, user_id: 92003, username: 's2', ready: true },
      ],
    });

    cy.openCutthroatGame(gameId, 'spectate');
    cy.get(CUTTHROAT_SELECTORS.deck).click();

    cy.request(`/cutthroat/api/v1/games/${gameId}/spectate/state`)
      .its('body')
      .then((before) => {
        cy.wait(200);
        cy.request(`/cutthroat/api/v1/games/${gameId}/spectate/state`)
          .its('body')
          .then((after) => {
            expect(after.version).to.eq(before.version);
          });
      });
  });
});
