describe('login', () => {
	it('shows the login form for an unauthenticated visitor', () => {
		cy.visit('/');
		cy.location('pathname').should('eq', '/login');
		cy.get('input[type="email"]').should('be.visible');
		cy.get('input[type="password"]').should('be.visible');
	});
});
