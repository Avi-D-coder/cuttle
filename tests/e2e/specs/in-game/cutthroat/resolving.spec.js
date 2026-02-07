import { tokenlogWithActions } from '../../../support/cutthroat/seed';

describe('Cutthroat 3P Resolving Phases', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('loads seeded resolving three phase with pick actions', () => {
    const gameId = 7351;
    const tokenlog = tokenlogWithActions({
      dealer: 'P2',
      actions: [
        'P0 MT_DRAW',
        'P1 MT_DRAW',
        'P2 MT_ONEOFF 3C',
        'P0 MT_CPASS',
        'P1 MT_CPASS',
      ],
    });

    cy.seedCutthroatGameFromTokenlog({ gameId, tokenlog, status: 1, playerSeat: 2 });
    cy.openCutthroatGame(gameId, 'game');

    cy.request(`/cutthroat/api/v1/games/${gameId}/state`)
      .its('body')
      .then((state) => {
        const actionTypes = state.legal_actions.map((action) => action.type);
        expect(actionTypes).to.include('ResolveThreePick');
      });
  });

  it('loads seeded resolving four phase with discard actions', () => {
    const gameId = 7352;
    const tokenlog = tokenlogWithActions({
      dealer: 'P2',
      actions: [
        'P0 MT_ONEOFF 4C TGT_P P1',
        'P1 MT_CPASS',
        'P2 MT_CPASS',
      ],
    });

    cy.seedCutthroatGameFromTokenlog({ gameId, tokenlog, status: 1, playerSeat: 1 });
    cy.openCutthroatGame(gameId, 'game');

    cy.request(`/cutthroat/api/v1/games/${gameId}/state`)
      .its('body')
      .then((state) => {
        const actionTypes = state.legal_actions.map((action) => action.type);
        expect(actionTypes).to.include('ResolveFourDiscard');
      });
  });
});
