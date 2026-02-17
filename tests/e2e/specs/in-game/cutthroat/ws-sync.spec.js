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

function waitForScrapStraightened(gameId, attemptsRemaining = 8) {
  return cy.request(`/cutthroat/api/v1/games/${gameId}/state`)
    .its('body.scrap_straightened')
    .then((scrapStraightened) => {
      if (scrapStraightened === true) {return;}
      if (attemptsRemaining <= 0) {
        throw new Error('Expected scrap_straightened to become true after websocket sync message.');
      }
      return waitForScrapStraightened(gameId, attemptsRemaining - 1);
    });
}

describe('Cutthroat 3P WS Sync', () => {
  beforeEach(() => {
    cy.setupCutthroatUser();
  });

  it('When the client sends a scrap-straighten websocket message in an active 3P game, then server state flips scrap_straightened to true because this UX sync action must persist through the authoritative runtime.', () => {
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
    waitForScrapStraightened(gameId);
  });
});
