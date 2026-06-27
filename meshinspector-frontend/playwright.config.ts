import { defineConfig, devices } from '@playwright/test';

const baseURL = process.env.MESHINSPECTOR_BASE_URL ?? 'http://127.0.0.1:48101';

export default defineConfig({
  testDir: './e2e',
  timeout: 180_000,
  expect: { timeout: 20_000 },
  retries: process.env.CI ? 1 : 0,
  fullyParallel: false,
  reporter: [
    ['list'],
    ['html', { outputFolder: '../docs/reports/meshinspector-ui-validation/playwright-html', open: 'never' }],
    ['json', { outputFile: '../docs/reports/meshinspector-ui-validation/playwright-results.json' }],
  ],
  use: {
    baseURL,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },
  projects: [
    {
      name: 'chromium-desktop',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 1440, height: 1000 },
      },
    },
  ],
});
