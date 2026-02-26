import { transcriptWithActions } from '../../../support/cutthroat/seed';
import { assertCutthroatBoardVisible } from '../../../support/cutthroat/assertions';

describe('Cutthroat 3P Routing', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('When an authenticated user is seated in a started 3P game and opens the game route, then the app keeps the game route and renders the board because seated players should land directly in gameplay.', () => {
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

  it('When an authenticated user is not seated in a started 3P game and opens the game route, then the app redirects to spectate and still renders the board because non-seated viewers must not join player mode implicitly.', () => {
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

  it('When Cutthroat capability checks fail and a user deep-links to a 3P route, then the app redirects to home and shows an unavailable message because 3P must fail closed when Rust is unavailable.', () => {
    cy.intercept('GET', '**/cutthroat/api/v1/health', {
      statusCode: 503,
      body: { alive: false },
    }).as('cutthroatHealthUnavailable');
    cy.window()
      .its('cuttle.capabilitiesStore')
      .then((store) => {
        store.cutthroatAvailability = 'unknown';
        store.cutthroatCheckedAt = 0;
        store.cutthroatNextRetryAt = 0;
      });

    cy.visit('/cutthroat/game/7309');

    cy.wait('@cutthroatHealthUnavailable');
    cy.location('pathname').should('eq', '/');
    cy.contains('Cutthroat is currently unavailable.').should('be.visible');
  });
});
