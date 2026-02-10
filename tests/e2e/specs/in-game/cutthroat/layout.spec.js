import { transcriptWithActions } from '../../../support/cutthroat/seed';
import { assertBottomGapWithin } from '../../../support/cutthroat/assertions';

describe('Cutthroat 3P Layout', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('anchors bottom area near viewport edge on desktop', () => {
    const gameId = 7321;
    const transcript = transcriptWithActions({ dealer: 'P2' });

    cy.seedCutthroatGameFromTranscript({ gameId,
      ...transcript, status: 1, playerSeat: 0 });

    cy.viewport(1440, 900);
    cy.openCutthroatGame(gameId, 'game');
    assertBottomGapWithin(24);
  });

  it('anchors bottom area near viewport edge on mobile', () => {
    const gameId = 7322;
    const transcript = transcriptWithActions({ dealer: 'P2' });

    cy.seedCutthroatGameFromTranscript({ gameId,
      ...transcript, status: 1, playerSeat: 0 });

    cy.viewport(390, 844);
    cy.openCutthroatGame(gameId, 'game');
    assertBottomGapWithin(18);
  });
});
