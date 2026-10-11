import { describe, expect, test } from 'vitest';
import { assignStableColors } from '../src/utils/chart-colors';

const PALETTE = ['red', 'green', 'blue'];

describe('assignStableColors', () => {
	test('assigns a distinct color per id', () => {
		const colors = assignStableColors(['a', 'b', 'c'], PALETTE);
		expect(new Set(colors.values()).size).toBe(3);
	});

	test('color follows identity, not input order', () => {
		const forward = assignStableColors(['a', 'b', 'c'], PALETTE);
		const shuffled = assignStableColors(['c', 'a', 'b'], PALETTE);
		expect(shuffled.get('a')).toBe(forward.get('a'));
		expect(shuffled.get('b')).toBe(forward.get('b'));
		expect(shuffled.get('c')).toBe(forward.get('c'));
	});

	test('duplicate ids collapse to one entry', () => {
		const colors = assignStableColors(['a', 'a', 'b'], PALETTE);
		expect(colors.size).toBe(2);
	});

	test('wraps around the palette past its length rather than erroring', () => {
		const colors = assignStableColors(['a', 'b', 'c', 'd'], PALETTE);
		expect(colors.size).toBe(4);
		expect([...colors.values()].every((c) => PALETTE.includes(c))).toBe(true);
	});
});
