import { CUTTHROAT_SELECTORS } from './selectors';

export function assertCutthroatBoardVisible() {
  cy.get(CUTTHROAT_SELECTORS.wrapper).should('be.visible');
  cy.get(CUTTHROAT_SELECTORS.deck).should('be.visible');
}

export function assertBottomGapWithin(maxGapPx) {
  cy.get(CUTTHROAT_SELECTORS.wrapper).should('exist');
  cy.get(CUTTHROAT_SELECTORS.tableBottom).should('exist');

  cy.window().then((win) => {
    const wrapper = win.document.querySelector(CUTTHROAT_SELECTORS.wrapper);
    const tableBottom = win.document.querySelector(CUTTHROAT_SELECTORS.tableBottom);
    const wrapperRect = wrapper.getBoundingClientRect();
    const tableBottomRect = tableBottom.getBoundingClientRect();
    const gap = wrapperRect.bottom - tableBottomRect.bottom;

    expect(gap).to.be.gte(-1);
    expect(gap).to.be.lte(maxGapPx);
  });
}

export function assertScrapAndHistoryBottomAligned(maxDeltaPx = 1) {
  cy.get(CUTTHROAT_SELECTORS.scrap).should('be.visible');
  cy.get(CUTTHROAT_SELECTORS.historyDesktop).should('be.visible');

  cy.window().then((win) => {
    const scrap = win.document.querySelector(CUTTHROAT_SELECTORS.scrap);
    const historyPanel = win.document.querySelector(CUTTHROAT_SELECTORS.historyDesktop);
    const scrapRect = scrap.getBoundingClientRect();
    const historyRect = historyPanel.getBoundingClientRect();
    const bottomDelta = Math.abs(scrapRect.bottom - historyRect.bottom);

    expect(bottomDelta).to.be.lte(maxDeltaPx);
  });
}
