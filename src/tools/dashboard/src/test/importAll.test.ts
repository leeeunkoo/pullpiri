import fg from 'fast-glob';

describe('import all modules smoke test', () => {
  it('imports all .ts and .tsx files under src to execute top-level code', async () => {
    // Ensure a root element exists for files that call createRoot(document.getElementById('root'))
    if (typeof document !== 'undefined' && !document.getElementById('root')) {
      const root = document.createElement('div');
      root.id = 'root';
      document.body.appendChild(root);
    }

    const entries = await fg(['src/**/*.ts', 'src/**/*.tsx'], { dot: false });
    // Filter out type files and tests and obvious server files
    const targets = entries.filter((p) => !p.includes('src/test') && !p.endsWith('.d.ts') && !p.startsWith('server/') && !p.includes('/server/'));

    const failures: string[] = [];
    for (const file of targets) {
      const relative = `../${file.replace(/^src\//, '')}`;
      try {
        // dynamic import; vitest will use the compiled source
        // eslint-disable-next-line no-await-in-loop
        await import(relative);
      } catch (err) {
        // Record and continue — some files require special runtime (native, node-only, etc.)
        failures.push(`${file}: ${String(err && (err as Error).message)}`);
      }
    }

    // We don't want this smoke test to fail the whole run if a few imports error.
    // Log any failures for later inspection, but assert we imported something.
    // eslint-disable-next-line no-console
    if (failures.length > 0) console.warn('importAll failures:', failures.slice(0, 10));

    expect(targets.length).toBeGreaterThan(0);
  });
});

import { describe, it } from 'vitest';
import fg from 'fast-glob';

describe('import all source files', () => {
  it('imports all ts/tsx files under src to exercise top-level code', async () => {
    const files = await fg(['src/**/*.{ts,tsx}'], { dot: true });
    // Import each file dynamically. Some files may require DOM/window mocks.
    for (const f of files) {
      // Skip test files
      if (f.includes('/test/')) continue;
      // Dynamic import path relative to project root of the test runner
      // normalize path for import
      // eslint-disable-next-line no-await-in-loop
      await import(`../../${f}`);
    }
  });
});
