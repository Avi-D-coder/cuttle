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

  it('When a point stack is contested by jack, joker, then jack, then all layered attachments remain visible on the controlled stack because mixed steal chains must preserve board readability.', () => {
    const gameId = 7324;
    const standardDeck = [
      'AC', '2C', '3C', '4C', '5C', '6C', '7C', '8C', '9C', 'TC', 'JC', 'QC', 'KC',
      'AD', '2D', '3D', '4D', '5D', '6D', '7D', '8D', '9D', 'TD', 'JD', 'QD', 'KD',
      'AH', '2H', '3H', '4H', '5H', '6H', '7H', '8H', '9H', 'TH', 'JH', 'QH', 'KH',
      'AS', '2S', '3S', '4S', '5S', '6S', '7S', '8S', '9S', 'TS', 'JS', 'QS', 'KS',
      'J0', 'J1',
    ];
    const prefix = [
      '4C', 'JC', 'J0',
      'JH', '2C', '3C',
      '5C', '6C', '7C',
      '8C', '9C', 'TC',
      'QC', 'KC', 'AD',
    ];
    const deckTokens = [
      ...prefix,
      ...standardDeck.filter((token) => !prefix.includes(token)),
    ];
    const transcript = transcriptWithActions({
      dealer: 'P2',
      deckTokens,
      actions: [
        'P0 points 4C',
        'P1 playRoyal JC 4C',
        'P2 playRoyal J0 JC',
        'P0 playRoyal JH 4C',
      ],
    });

    cy.seedCutthroatGameFromTranscript({
      gameId,
      ...transcript,
      status: 1,
      playerSeat: 0,
    });

    cy.viewport(1440, 900);
    cy.openCutthroatGame(gameId, 'game');
    cy.get('.table-bottom [data-cutthroat-point-card="4C"]').should('be.visible');
    cy.get('.table-bottom [data-cutthroat-jack-card="JC"]').should('be.visible');
    cy.get('.table-bottom [data-cutthroat-joker-card="J0"]').should('be.visible');
    cy.get('.table-bottom [data-cutthroat-jack-card="JH"]').should('be.visible');
  });
});
