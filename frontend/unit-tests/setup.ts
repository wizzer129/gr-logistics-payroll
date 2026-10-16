// Node 25+ ships its own `localStorage` global, a stub without `getItem` unless Node is
// started with `--localstorage-file`, and it shadows happy-dom's. Swap in happy-dom's.
if (typeof globalThis.localStorage?.getItem !== 'function') {
	Object.defineProperty(globalThis, 'localStorage', {
		value: new Storage(),
		configurable: true,
		writable: true,
	});
}
