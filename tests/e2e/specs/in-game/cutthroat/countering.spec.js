import { tokenlogWithActions } from '../../../support/cutthroat/seed';

describe('Cutthroat 3P Countering Phase', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('loads a seeded countering phase with counter actions for current seat', () => {
    const gameId = 7341;
    const tokenlog = tokenlogWithActions({
      dealer: 'P2',
      actions: [
        'P0 MT_ONEOFF 4C TGT_P P1',
      ],
    });

    cy.seedCutthroatGameFromTokenlog({
      gameId,
      tokenlog,
      status: 1,
      playerSeat: 1,
    });

    cy.openCutthroatGame(gameId, 'game');

    cy.request(`/cutthroat/api/v1/games/${gameId}/state`)
      .its('body')
      .then((state) => {
        const actionTypes = state.legal_actions.map((action) => action.type);
        expect(actionTypes).to.include('CounterPass');
        expect(actionTypes).to.include('CounterTwo');
      });
  });
});
