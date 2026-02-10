import { transcriptWithActions } from '../../../support/cutthroat/seed';

function installCutthroatWsTracker(win) {
  const NativeWebSocket = win.WebSocket;
  if (!NativeWebSocket || win.__cutthroatWsTrackerInstalled) {return;}

  win.__cutthroatWsInstances = [];
  win.__cutthroatWsTrackerInstalled = true;

  win.WebSocket = class TrackedWebSocket extends NativeWebSocket {
    constructor(url, protocols) {
      super(url, protocols);
      win.__cutthroatWsInstances.push({
        instance: this,
        url: typeof url === 'string' ? url : String(url),
      });
    }
  };
}

describe('Cutthroat 3P WS Sync', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('toggles scrap straighten state via websocket message', () => {
    const gameId = 7361;
    const transcript = transcriptWithActions({
      dealer: 'P2',
      actions: [
        'P0 oneOff AC',
        'P1 resolve',
        'P2 resolve',
      ],
    });

    cy.seedCutthroatGameFromTranscript({ gameId,
      ...transcript, status: 1, playerSeat: 0 });
    cy.visit(`/cutthroat/game/${gameId}`, {
      onBeforeLoad(win) {
        installCutthroatWsTracker(win);
      },
    });
    cy.get('#cutthroat-game-wrapper').should('be.visible');

    cy.window().should((win) => {
      const ws = win.__cutthroatWsInstances
        .find((entry) => entry.url.includes(`/cutthroat/ws/games/${gameId}`))
        ?.instance;
      expect(ws).to.exist;
      expect(ws.readyState).to.eq(1);
    });

    cy.window().then((win) => {
      const ws = win.__cutthroatWsInstances
        .find((entry) => entry.url.includes(`/cutthroat/ws/games/${gameId}`))
        ?.instance;
      ws.send(JSON.stringify({ type: 'scrap_straighten' }));
    });
    cy.wait(250);

    cy.request(`/cutthroat/api/v1/games/${gameId}/state`)
      .its('body')
      .then((state) => {
        expect(state.scrap_straightened).to.eq(true);
      });
  });
});
