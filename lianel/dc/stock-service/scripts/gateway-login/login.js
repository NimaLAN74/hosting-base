#!/usr/bin/env node
/**
 * Headless login to IBKR Client Portal Gateway. Prints the session cookie value
 * to stdout so it can be set as IBKR_GATEWAY_SESSION_COOKIE.
 *
 * Usage:
 *   IBKR_USERNAME=your_username IBKR_PASSWORD=your_password node login.js
 *   GATEWAY_URL=https://localhost:5000 (optional, default https://localhost:5000)
 *
 * Requires: npx playwright install chromium (once)
 */

const { chromium } = require('playwright');

const GATEWAY_URL = process.env.GATEWAY_URL || 'https://localhost:5000';
const USERNAME = process.env.IBKR_USERNAME;
const PASSWORD = process.env.IBKR_PASSWORD;

if (!USERNAME || !PASSWORD) {
  console.error('Set IBKR_USERNAME and IBKR_PASSWORD');
  process.exit(1);
}

(async () => {
  let browser;
  try {
    browser = await chromium.launch({ headless: true });
    const context = await browser.newContext({
      ignoreHTTPSErrors: true,
      userAgent: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36',
    });
    const page = await context.newPage();

    await page.goto(GATEWAY_URL, { waitUntil: 'networkidle', timeout: 30000 });

    // IBKR Gateway login form: common selectors
    const usernameSel = 'input[name="username"], input[id="username"], input[type="text"]';
    const passwordSel = 'input[name="password"], input[id="password"], input[type="password"]';
    const submitSel = 'input[type="submit"], button[type="submit"], button:has-text("Log in"), .kc-button';

    await page.waitForSelector(usernameSel, { timeout: 10000 });
    await page.fill(usernameSel, USERNAME);
    await page.fill(passwordSel, PASSWORD);
    await page.click(submitSel);

    // Wait for navigation (success or 2FA page)
    await page.waitForTimeout(5000);
    const url = page.url();
    if (url.includes('login') || url.includes('auth')) {
      // Maybe 2FA or error - wait a bit more and then try to get cookies anyway
      await page.waitForTimeout(5000);
    }

    const cookies = await context.cookies();
    const apiCookie = cookies.find((c) => c.name === 'api' || c.name === 'JSESSIONID' || c.name === 'session');
    if (apiCookie) {
      console.log(apiCookie.value);
    } else {
      // Print all cookie names for debugging
      const names = cookies.map((c) => c.name).join(', ');
      console.error('No "api" or "JSESSIONID" cookie found. Cookies:', names || 'none');
      process.exit(1);
    }
  } catch (e) {
    console.error(e.message || e);
    process.exit(1);
  } finally {
    if (browser) await browser.close();
  }
})();
