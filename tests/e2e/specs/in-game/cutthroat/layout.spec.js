import { transcriptWithActions } from '../../../support/cutthroat/seed';
import { assertBottomGapWithin } from '../../../support/cutthroat/assertions';

describe('Cutthroat 3P Layout', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('When the 3P board renders on desktop, then the local player area stays anchored near the viewport bottom because local-hand interaction depends on consistent spatial placement.', () => {
    const gameId = 7321;
    const transcript = transcriptWithActions({ dealer: 'P2' });

    cy.seedCutthroatGameFromTranscript({ gameId,
      ...transcript, status: 1, playerSeat: 0 });

    cy.viewport(1440, 900);
    cy.openCutthroatGame(gameId, 'game');
    assertBottomGapWithin(24);
  });

  it('When the 3P board renders on mobile, then the local player area still stays anchored near the viewport bottom because responsive layout must preserve touch ergonomics.', () => {
    const gameId = 7322;
    const transcript = transcriptWithActions({ dealer: 'P2' });

    cy.seedCutthroatGameFromTranscript({ gameId,
      ...transcript, status: 1, playerSeat: 0 });

    cy.viewport(390, 844);
    cy.openCutthroatGame(gameId, 'game');
    assertBottomGapWithin(18);
  });

  it('When a jack steals control of a local point stack, then both the base point and jack attachment remain visible in the local area because ownership context must stay readable after control changes.', () => {
    const gameId = 7323;
    const transcript = transcriptWithActions({
      dealer: 'P2',
      actions: [
        'P0 points 4C',
        'P1 playRoyal JC 4C',
      ],
    });

    cy.seedCutthroatGameFromTranscript({
      gameId,
      ...transcript,
      status: 1,
      playerSeat: 1,
    });

    cy.viewport(1440, 900);
    cy.openCutthroatGame(gameId, 'game');
    cy.get('.table-bottom [data-cutthroat-point-card="4C"]').should('be.visible');
    cy.get('.table-bottom [data-cutthroat-jack-card="JC"]').should('be.visible');
  });
});
