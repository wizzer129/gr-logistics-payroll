// Builds the email templates into the backend, which embeds them and fills in the
// `{{ }}` placeholders at send time (see EMAIL-SPEC.md §3). Output is committed.
const BLOCK_TAGS = new Set(['p', 'h1', 'h2', 'h3', 'div', 'tr', 'br', 'li', 'table']);

export default {
	output: { path: '../backend/templates/email' },
	plaintext: {
		options: {
			// Maizzle's HTML is one long line, so without this every paragraph runs together.
			cb: ({ tag, deleteFrom, deleteTo, rangesArr, proposedReturn }) => {
				if (BLOCK_TAGS.has(tag.name?.toLowerCase())) {
					rangesArr.push(deleteFrom, deleteTo, '\n');
				} else if (proposedReturn) {
					rangesArr.push(...proposedReturn);
				}
			},
		},
	},
};
