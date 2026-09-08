import { defineConfig, devices } from "@playwright/test"

const baseURL = process.env.MASK_BASE_URL ?? "http://127.0.0.1:8000"

export default defineConfig({
  testDir: "./tests",
  use: {
    baseURL,
    trace: "retain-on-failure",
  },
  webServer: process.env.MASK_BASE_URL
    ? undefined
    : {
        command: "cargo run -- serve --port 8000",
        cwd: "..",
        url: `${baseURL}/health`,
        reuseExistingServer: true,
        timeout: 120_000,
      },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
})
