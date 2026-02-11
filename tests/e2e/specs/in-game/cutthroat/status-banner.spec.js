import { transcriptWithActions } from '../../../support/cutthroat/seed';
import { CUTTHROAT_SELECTORS } from '../../../support/cutthroat/selectors';

describe('Cutthroat 3P Status Banner', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('renders points/goal for all seats and turn status only for the local seat', () => {
    const gameId = 7371;
    const transcript = transcriptWithActions({ dealer: 'P2' });

    cy.seedCutthroatGameFromTranscript({
      gameId,
      ...transcript,
      status: 1,
      playerSeat: 0,
    });

    cy.openCutthroatGame(gameId, 'game');
    cy.get(CUTTHROAT_SELECTORS.seatStatuses).should('have.length', 3);
    cy.get(CUTTHROAT_SELECTORS.seatPoints).should('have.length', 3);
    cy.get(CUTTHROAT_SELECTORS.seatGoals).should('have.length', 3);
    cy.get(CUTTHROAT_SELECTORS.seatTurns).should('have.length', 1);
    cy.get('.player-area.active-turn').should('have.length', 1);

    cy.request(`/cutthroat/api/v1/games/${gameId}/state`)
      .its('body')
      .then((state) => {
        const activeSeat = state.player_view.turn;
        const localSeat = 0;
        const expectedMyTurnCount = activeSeat === localSeat ? 1 : 0;

        cy.get(`${CUTTHROAT_SELECTORS.turnIndicator}.my-turn`).should('have.length', expectedMyTurnCount);
        [ 0, 1, 2 ].forEach((seat) => {
          cy.get(`[data-cy=cutthroat-seat-points-${seat}]`).should('contain', 'POINTS');
          cy.get(`[data-cy=cutthroat-seat-goal-${seat}]`).should('contain', 'GOAL');
        });

        const expectedTurnLabel = activeSeat === localSeat
          ? 'YOUR TURN'
          : 'OPPONENT\'S TURN';
        cy.get(`[data-cy=cutthroat-seat-turn-${localSeat}]`).should('contain', expectedTurnLabel);
        [ 1, 2 ].forEach((seat) => {
          cy.get(`[data-cy=cutthroat-seat-turn-${seat}]`).should('not.exist');
        });
      });
  });
});
