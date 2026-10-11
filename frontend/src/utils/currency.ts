// Money is always displayed in en-US punctuation/symbol conventions ($1,234.56) regardless
// of the active UI language — using vue-i18n's n() for this would format USD with Spanish
// punctuation under the es locale (e.g. "1.234,56 US$"), which reads as a different currency
// to a US-based business.
const formatter = new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' });

export function formatCurrency(value: number): string {
	return formatter.format(value);
}
