import { transcriptWithActions } from '../../../support/cutthroat/seed';

describe('Cutthroat 3P Resolving Phases', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('When a seeded transcript enters resolving-three, then legal actions still include resolve choices for the acting seat because resolve chains must preserve deterministic completion options.', () => {
    const gameId = 7351;
    const transcript = transcriptWithActions({
      dealer: 'P2',
      actions: [
        'P0 draw',
        'P1 draw',
        'P2 oneOff 3C',
        'P0 resolve',
        'P1 resolve',
      ],
    });

    cy.seedCutthroatGameFromTranscript({ gameId,
      ...transcript, status: 1, playerSeat: 2 });
    cy.openCutthroatGame(gameId, 'game');

    cy.request(`/cutthroat/api/v1/games/${gameId}/state`)
      .its('body')
      .then((state) => {
        expect([ 'ResolvingThree', 'Main' ]).to.include(state.view.phase.type);
        if (state.view.phase.type === 'ResolvingThree') {
          expect(state.legal_actions.some((token) => /\sresolve\b/.test(token))).to.equal(true);
        }
      });
  });

  it('When a seeded transcript enters resolving-four, then legal actions include typed discard resolves because resolving-four requires explicit discard token selection to advance state.', () => {
    const gameId = 7352;
    const transcript = transcriptWithActions({
      dealer: 'P2',
      actions: [
        'P0 oneOff 4C P1',
        'P1 resolve',
        'P2 resolve',
      ],
    });

    cy.seedCutthroatGameFromTranscript({ gameId,
      ...transcript, status: 1, playerSeat: 1 });
    cy.openCutthroatGame(gameId, 'game');

    cy.request(`/cutthroat/api/v1/games/${gameId}/state`)
      .its('body')
      .then((state) => {
        const hasResolveFourDiscard = state.legal_actions.some((token) => {
          return /\sresolve\s+discard\s+[A2-9TJQK][CDHS]$/.test(token);
        });
        expect(hasResolveFourDiscard).to.equal(true);
      });
  });
});
