import { transcriptWithActions } from '../../../support/cutthroat/seed';

describe('Cutthroat 3P Countering Phase', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('When a one-off puts the game into countering phase for the local seat, then legal actions include both counter and resolve options because counter windows must expose all valid responses.', () => {
    const gameId = 7341;
    const transcript = transcriptWithActions({
      dealer: 'P2',
      actions: [
        'P0 oneOff 4C P1',
      ],
    });

    cy.seedCutthroatGameFromTranscript({
      gameId,
      ...transcript,
      status: 1,
      playerSeat: 1,
    });

    cy.openCutthroatGame(gameId, 'game');

    cy.request(`/cutthroat/api/v1/games/${gameId}/state`)
      .its('body')
      .then((state) => {
        expect(state.legal_actions.some((token) => token.endsWith(' resolve'))).to.equal(true);
        expect(state.legal_actions.some((token) => /\scounter\s+[A2-9TJQK][CDHS]$/.test(token))).to.equal(true);
      });
  });
});
