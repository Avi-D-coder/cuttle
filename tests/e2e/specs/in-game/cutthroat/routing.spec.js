import { transcriptWithActions } from '../../../support/cutthroat/seed';
import { assertCutthroatBoardVisible } from '../../../support/cutthroat/assertions';

describe('Cutthroat 3P Routing', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('loads seeded game route for seated player', () => {
    const gameId = 7301;
    const transcript = transcriptWithActions({ dealer: 'P2' });

    cy.seedCutthroatGameFromTranscript({
      gameId,
      ...transcript,
      status: 1,
      playerSeat: 0,
    });

    cy.openCutthroatGame(gameId, 'game');
    cy.url().should('include', `/cutthroat/game/${gameId}`);
    assertCutthroatBoardVisible();
  });

  it('redirects /game route to /spectate when authenticated user is not seated', () => {
    const gameId = 7302;
    const transcript = transcriptWithActions({ dealer: 'P2' });

    cy.seedCutthroatGameFromTranscript({
      gameId,
      ...transcript,
      status: 1,
      players: [
        { seat: 0, user_id: 91001, username: 'r0', ready: true },
        { seat: 1, user_id: 91002, username: 'r1', ready: true },
        { seat: 2, user_id: 91003, username: 'r2', ready: true },
      ],
    });

    cy.openCutthroatGame(gameId, 'game');
    cy.url().should('include', `/cutthroat/spectate/${gameId}`);
    assertCutthroatBoardVisible();
  });
});
