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
  await byTestId(page, "shell-toggle-view").click();
  await expectNow(byTestId(page, "shell-status"), "Connected to MOCK_JUKEBOY");
}

test("mock backend propagates every visible component state immediately", async ({ page }) => {
  await connectMock(page);
  await byTestId(page, "shell-toggle-view").click();

  await expectNow(byTestId(page, "dashboard-track-title"), "Signal Mirror");
  await expectNow(byTestId(page, "strip-title"), "Signal Mirror");
  await expectNow(byTestId(page, "strip-artist"), "Test Pressing");
  await expectNow(byTestId(page, "library-track-count"), "5");

  await byTestId(page, "strip-next").click();
  await expectNow(byTestId(page, "dashboard-track-title"), "Immediate Event");
  await expectNow(byTestId(page, "strip-title"), "Immediate Event");

  await byTestId(page, "strip-next").click();
  await expectNow(byTestId(page, "dashboard-track-title"), "Heartbeat Window");
  await expectNow(byTestId(page, "strip-title"), "Heartbeat Window");

  await byTestId(page, "strip-repeat").click();
  await expectNow(byTestId(page, "dashboard-mode"), "Shuffle");

  await byTestId(page, "strip-output-menu").click();
  await byTestId(page, "strip-output-bluetooth").click();
  await expectNow(byTestId(page, "dashboard-output"), "Bluetooth");

  await byTestId(page, "track-item-4").scrollIntoViewIfNeeded();
  await byTestId(page, "track-item-4").click();
  await expectNow(byTestId(page, "dashboard-track-title"), "Pairing Sequence");
  await expectNow(byTestId(page, "strip-title"), "Pairing Sequence");
  await expectNow(byTestId(page, "strip-artist"), "Button Matrix");

  await byTestId(page, "shell-toggle-view").click();
  await expectNow(byTestId(page, "pairing-pending"), "No");
  await expect(byTestId(page, "pairing-progress")).toContainText("0 / 4", { timeout: immediateTimeout });
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
  await expectNow(byTestId(page, "bt-output"), "Bluetooth");

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

  await page.getByText("Smoke test panel").click();
  await expect(page.getByText("companion_auth")).toHaveCount(0);
  await expect(page.getByText("companion_trusted_revoke")).toHaveCount(0);
});