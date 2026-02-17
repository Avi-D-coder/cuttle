import { transcriptWithActions } from '../../../support/cutthroat/seed';
import { CUTTHROAT_SELECTORS } from '../../../support/cutthroat/selectors';

describe('Cutthroat 3P Basic Moves', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('When the local seat is in a legal main phase and draws from deck, then game state version increments by one because each legal action must produce exactly one authoritative state update.', () => {
    const gameId = 7311;
    const transcript = transcriptWithActions({ dealer: 'P2' });

    cy.seedCutthroatGameFromTranscript({
      gameId,
      ...transcript,
      status: 1,
      playerSeat: 0,
    });

    cy.openCutthroatGame(gameId, 'game');

    cy.request(`/cutthroat/api/v1/games/${gameId}/state`)
      .its('body')
      .then((before) => {
        cy.get(CUTTHROAT_SELECTORS.deck).click();
        cy.request(`/cutthroat/api/v1/games/${gameId}/state`)
          .its('body')
          .then((after) => {
            expect(after.version).to.eq(before.version + 1);
          });
      });
  });

  it('When the local seat selects a hand card and chooses the points move, then the card appears in that seat point stacks because point-play resolution must be reflected in persisted game state.', () => {
    const gameId = 7312;
    const transcript = transcriptWithActions({ dealer: 'P2' });

    cy.seedCutthroatGameFromTranscript({
      gameId,
      ...transcript,
      status: 1,
      playerSeat: 0,
    });

    cy.openCutthroatGame(gameId, 'game');

    cy.get('[data-cutthroat-hand-card="4C"]').click();
    cy.get('[data-cy=cutthroat-move-choice-points]').click();

    cy.request(`/cutthroat/api/v1/games/${gameId}/state`)
      .its('body')
      .then((state) => {
        expect(state.version).to.eq(1);
        expect(state.view.players[0].points.length).to.eq(1);
      });
  });

  it('When the local seat executes a legal scuttle against an opponent point card, then both the target card and scuttling card move to scrap because scuttle consumes both cards by rule.', () => {
    const gameId = 7313;
    const transcript = transcriptWithActions({
      dealer: 'P2',
      actions: [
        'P0 draw',
        'P1 points 2C',
        'P2 draw',
      ],
    });

    cy.seedCutthroatGameFromTranscript({
      gameId,
      ...transcript,
      status: 1,
      playerSeat: 0,
    });

    cy.openCutthroatGame(gameId, 'game');

    cy.get('[data-cutthroat-hand-card="4C"]').click();
    cy.get('[data-cy=cutthroat-move-choice-scuttle]').click();
    cy.get('[data-cutthroat-point-card="2C"]').click();

    cy.request(`/cutthroat/api/v1/games/${gameId}/state`)
      .its('body')
      .then((state) => {
        expect(state.version).to.eq(4);
        expect(state.view.scrap).to.include('2C');
        expect(state.view.scrap).to.include('4C');
      });
  });
});
