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
    browser = await chromium.launch({
      headless: true,
      args: ['--no-sandbox', '--disable-dev-shm-usage', '--disable-gpu'],
    });
    const context = await browser.newContext({
      ignoreHTTPSErrors: true,
      userAgent: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36',
    });
    const page = await context.newPage();

    await page.goto(GATEWAY_URL, { waitUntil: 'domcontentloaded', timeout: 45000 });
    await page.waitForTimeout(25000);
    let inputCount = await page.evaluate(() => document.querySelectorAll('input').length).catch(() => 0);
    if (inputCount === 0) {
      await page.waitForTimeout(40000);
      inputCount = await page.evaluate(() => document.querySelectorAll('input').length).catch(() => 0);
    }
    if (inputCount === 0) {
      console.error('No inputs found. Page may require full JS load or different selectors.');
      process.exit(1);
    }
    await page.waitForSelector('input[type="text"], input[name="username"], input[name="userId"], .loginformWrapper input', { timeout: 20000 });

    const usernameSel = '.loginformWrapper input[type="text"], input[type="text"]';
    const passwordSel = '.loginformWrapper input[type="password"], input[type="password"]';
    const submitSel = '.loginformWrapper button[type="submit"], .loginformWrapper input[type="submit"], button[type="submit"], input[type="submit"]';

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
