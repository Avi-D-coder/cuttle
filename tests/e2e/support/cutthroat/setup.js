import { myUser } from '../../fixtures/userFixtures';
import { announcementData } from '../../../../src/routes/home/components/announcementDialog/data/announcementData';

export function setupCutthroatUser() {
  cy.wipeDatabase();
  cy.visit('/', {
    onBeforeLoad(win) {
      win.localStorage.setItem('announcement', announcementData.id);
    },
  });
  cy.signupPlayer(myUser);

  return cy.request('/api/user/status')
    .its('body')
    .then((status) => {
      expect(status.authenticated).to.eq(true);
      expect(status.id).to.be.a('number');
      expect(status.username).to.be.a('string');

      const user = {
        id: status.id,
        username: status.username,
      };
      cy.wrap(user, { log: false }).as('cutthroatUser');
      return cy.wrap(user, { log: false });
    });
}
