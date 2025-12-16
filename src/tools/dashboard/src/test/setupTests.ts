import '@testing-library/jest-dom';

// Minimal matchMedia mock for jsdom environment used by vitest
if (typeof window !== 'undefined' && !window.matchMedia) {
	// @ts-ignore
	window.matchMedia = (query: string) => ({
		matches: false,
		media: query,
		onchange: null,
		addEventListener: () => {},
		removeEventListener: () => {},
		addListener: () => {},
		removeListener: () => {},
		dispatchEvent: () => false,
	});
}

// Mock ResizeObserver used by some charting libraries
if (typeof (globalThis as any).ResizeObserver === 'undefined') {
	(globalThis as any).ResizeObserver = class {
		observe() {}
		unobserve() {}
		disconnect() {}
	};
}

// Stub setInterval globally to prevent real repeating timers from keeping the process alive
// Tests can still clearInterval with the returned id.
// @ts-ignore
if (typeof globalThis.setInterval === 'function') {
	// keep the original in case someone wants to restore
	try {
		// @ts-ignore
		(globalThis as any).__original_setInterval = globalThis.setInterval
	} catch (e) {}
}
// @ts-ignore
globalThis.setInterval = (fn: any, _t: any, ..._args: any[]) => {
	// do not schedule the callback — return a dummy id
	return 42 as unknown as number
}

// Always stub global fetch to avoid real network calls during tests
// @ts-ignore
globalThis.fetch = async (_input?: any, _init?: any) => {
	return {
		ok: true,
		json: async () => ([]),
		text: async () => '',
	} as any
}

// Simple module mocks to avoid executing heavy library code at import time
// Note: vitest will resolve these moduleNameMapper-style; here we attach simple noop implementations
const noop = () => null;

// Provide simple stubs for common UI libs used in the dashboard
try {
	// @ts-ignore
	const recharts = require.resolve('recharts');
	// if recharts exists, create no-op exports (only if module resolution works)
	// but avoid overwriting real package; we'll mock via vitest when needed in tests
} catch (e) {
	// no-op
}

// Diagnostic teardown removed — it could interfere with vitest lifecycle.


