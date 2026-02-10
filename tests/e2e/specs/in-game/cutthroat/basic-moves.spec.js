import { transcriptWithActions } from '../../../support/cutthroat/seed';
import { CUTTHROAT_SELECTORS } from '../../../support/cutthroat/selectors';

describe('Cutthroat 3P Basic Moves', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('draws from deck when action is legal', () => {
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

  it('plays a points card from hand', () => {
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
        expect(state.player_view.players[0].points.length).to.eq(1);
      });
  });

  it('scuttles an opponent point stack in a seeded turn-state', () => {
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
        expect(state.player_view.scrap).to.include('2C');
        expect(state.player_view.scrap).to.include('4C');
      });
  });
});
