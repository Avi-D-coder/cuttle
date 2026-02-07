import { tokenlogWithActions } from '../../../support/cutthroat/seed';
import { assertBottomGapWithin } from '../../../support/cutthroat/assertions';

describe('Cutthroat 3P Layout', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('anchors bottom area near viewport edge on desktop', () => {
    const gameId = 7321;
    const tokenlog = tokenlogWithActions({ dealer: 'P2' });

    cy.seedCutthroatGameFromTokenlog({ gameId, tokenlog, status: 1, playerSeat: 0 });

    cy.viewport(1440, 900);
    cy.openCutthroatGame(gameId, 'game');
    assertBottomGapWithin(24);
  });

  it('anchors bottom area near viewport edge on mobile', () => {
    const gameId = 7322;
    const tokenlog = tokenlogWithActions({ dealer: 'P2' });

    cy.seedCutthroatGameFromTokenlog({ gameId, tokenlog, status: 1, playerSeat: 0 });

    cy.viewport(390, 844);
    cy.openCutthroatGame(gameId, 'game');
    assertBottomGapWithin(18);
  });
});
