import { expect, test, type Locator, type Page } from "@playwright/test";

const immediateTimeout = 500;

function byTestId(page: Page, testId: string): Locator {
  return page.locator(`[data-testid="${testId}"]`);
}

async function expectNow(locator: Locator, text: string | RegExp): Promise<void> {
  await expect(locator).toContainText(text, { timeout: immediateTimeout });
}

async function connectMock(page: Page): Promise<void> {
  await page.goto("/?mock=1");
  await expect(byTestId(page, "shell-status")).toContainText("Connected to MOCK_JUKEBOY", { timeout: 2_000 });
  await byTestId(page, "shell-toggle-view").click();
}

test("mock backend propagates every visible component state immediately", async ({ page }) => {
  await connectMock(page);
  await byTestId(page, "shell-toggle-view").click();

  await expectNow(byTestId(page, "dashboard-track-title"), "Signal Mirror");
  await expectNow(byTestId(page, "strip-title"), "Signal Mirror");
  await expectNow(byTestId(page, "strip-artist"), "Test Pressing");
  await expectNow(byTestId(page, "library-track-count"), "5");

  await byTestId(page, "strip-output-menu").click();
  await expect(byTestId(page, "strip-output-bluetooth")).toBeDisabled({ timeout: immediateTimeout });
  await page.keyboard.press("Escape");

  await byTestId(page, "strip-next").click();
  await expectNow(byTestId(page, "dashboard-track-title"), "Immediate Event");
  await expectNow(byTestId(page, "strip-title"), "Immediate Event");

  await byTestId(page, "strip-next").click();
  await expectNow(byTestId(page, "dashboard-track-title"), "Heartbeat Window");
  await expectNow(byTestId(page, "strip-title"), "Heartbeat Window");

  await byTestId(page, "track-item-4").scrollIntoViewIfNeeded();
  await byTestId(page, "track-item-4").click();
  await expectNow(byTestId(page, "dashboard-track-title"), "Pairing Sequence");
  await expectNow(byTestId(page, "strip-title"), "Pairing Sequence");
  await expectNow(byTestId(page, "strip-artist"), "Button Matrix");

  await byTestId(page, "shell-toggle-view").click();
  await expect(byTestId(page, "pairing-progress")).toHaveCount(0);
  await expect(byTestId(page, "pairing-begin")).toHaveCount(0);
  await expect(byTestId(page, "pairing-cancel")).toHaveCount(0);

  await byTestId(page, "wifi-disconnect").scrollIntoViewIfNeeded();
  await byTestId(page, "wifi-disconnect").click();
  await expectNow(byTestId(page, "wifi-state"), "Disconnected");
  await expectNow(byTestId(page, "wifi-internet"), "Offline");

  await byTestId(page, "wifi-scan").click();
  await expect(page.getByText("Jukeboy Lab")).toBeVisible({ timeout: immediateTimeout });

  await byTestId(page, "wifi-connect").click();
  await expectNow(byTestId(page, "wifi-state"), "Connected");
  await expectNow(byTestId(page, "wifi-internet"), "Online");

  await page.getByLabel("Scrobbling").uncheck({ force: true });
  await expect(page.getByLabel("Scrobbling")).not.toBeChecked({ timeout: immediateTimeout });

  await byTestId(page, "bt-connect-last").scrollIntoViewIfNeeded();
  await byTestId(page, "bt-connect-last").click();
  await expectNow(byTestId(page, "bt-status"), "A2DP connected");

  await byTestId(page, "strip-output-menu").click();
  await expect(byTestId(page, "strip-output-bluetooth")).toBeEnabled({ timeout: immediateTimeout });
  await page.keyboard.press("Escape");

  await byTestId(page, "bt-disconnect").click();
  await expectNow(byTestId(page, "bt-status"), "Idle");
});

test("advanced connection fields are hidden from end-user and debug surfaces", async ({ page }) => {
  await connectMock(page);

  await expect(page.getByLabel("Profile")).toHaveCount(0);
  await expect(page.getByLabel("Client ID")).toHaveCount(0);
  await expect(page.getByLabel("App Name")).toHaveCount(0);
  await expect(page.getByLabel("Shared Secret")).toHaveCount(0);
  await expect(page.getByLabel("Button Sequence")).toHaveCount(0);
  await expect(page.getByLabel("Auth URL")).toHaveCount(0);
  await expect(page.getByText("Hardware Info")).toHaveCount(0);
  await expect(page.getByText("Protocol Version")).toHaveCount(0);
  await expect(page.getByText("MTU")).toHaveCount(0);
  await expect(page.getByText("Generation")).toHaveCount(0);
  await expect(page.getByText("Uptime")).toHaveCount(0);
  await expect(page.getByText("Smoke test panel")).toHaveCount(0);
  await expect(page.getByText("RSSI")).toHaveCount(0);
  await expect(page.getByText("Channel")).toHaveCount(0);
  await expect(page.getByText("IP")).toHaveCount(0);
  await expect(page.getByText("Bonded")).toHaveCount(0);
  await expect(page.getByText("MO:CK:BE:EF:00:01")).toHaveCount(0);
  await expect(page.getByText("companion_auth")).toHaveCount(0);
  await expect(page.getByText("companion_trusted_revoke")).toHaveCount(0);
});

test("settings exposes scripts while keeping advanced maintenance collapsed", async ({ page }) => {
  await connectMock(page);

  await expect(byTestId(page, "settings-scripts-section")).toBeVisible({ timeout: immediateTimeout });
  await expectNow(byTestId(page, "script-item-0"), "Refresh Artwork Cache");
  await expect(page.getByText("Backend Smoke Console")).toHaveCount(0);

  await byTestId(page, "script-run-0").click();
  await expectNow(byTestId(page, "script-log-output"), /Refresh Artwork Cache|Completed successfully/);

  await page.getByRole("button", { name: /Debug & Maintenance/i }).click();
  await expect(page.getByText("Backend Smoke Console")).toBeVisible({ timeout: immediateTimeout });
});

test("connection gate blocks the app until reconnect and returns after a link drop", async ({ page }) => {
  await connectMock(page);

  await byTestId(page, "settings-disconnect").click();

  await expectNow(byTestId(page, "shell-status"), "Connection paused");
  await expect(byTestId(page, "connection-gate")).toBeVisible({ timeout: immediateTimeout });
  await expect(byTestId(page, "shell-toggle-view")).toHaveCount(0);
  await expect(byTestId(page, "strip-title")).toHaveCount(0);
  await expect(page.getByRole("button", { name: /^close$/i })).toHaveCount(0);

  await byTestId(page, "connection-gate-scan").click();

  await expect(byTestId(page, "connection-gate")).toHaveCount(0, { timeout: 2_000 });
  await expect(byTestId(page, "shell-status")).toContainText("Connected to MOCK_JUKEBOY", { timeout: 2_000 });
  await expect(
    page.locator("[data-testid='shell-notification-recovery']").filter({ hasText: "Auto-connect resumed" }),
  ).toBeVisible({ timeout: immediateTimeout });
  await expect(
    page.locator("[data-testid='shell-notification-recovery']").filter({ hasText: "Companion connected" }),
  ).toBeVisible({ timeout: immediateTimeout });

  await page.evaluate(() => window.dispatchEvent(new Event("jukeboy:mock-disconnect")));
  await expect(byTestId(page, "connection-gate")).toBeVisible({ timeout: immediateTimeout });
  await expect(byTestId(page, "strip-title")).toHaveCount(0);

  await page.evaluate(() => window.dispatchEvent(new Event("jukeboy:mock-restore")));
  await byTestId(page, "connection-gate-scan").click();
  await byTestId(page, "connection-gate-connect-device").click();
  await expect(byTestId(page, "connection-gate")).toHaveCount(0, { timeout: 2_000 });
  await expect(byTestId(page, "shell-status")).toContainText("Connected to MOCK_JUKEBOY", { timeout: 2_000 });
});