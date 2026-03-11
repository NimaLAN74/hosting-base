# One-off run to obtain IBKR Gateway session cookie. Use with --network host so container can reach Gateway on host.
# docker build -t ibkr-gateway-login . && docker run --rm --network host -e IBKR_USERNAME -e IBKR_PASSWORD ibkr-gateway-login
FROM node:20-bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends libnss3 libnspr4 libatk1.0-0 libatk-bridge2.0-0 libcups2 libdrm2 libxkbcommon0 libxcomposite1 libxdamage1 libxfixes3 libxrandr2 libgbm1 libasound2 && rm -rf /var/lib/apt/lists/*
COPY package.json login.js ./
RUN npm install --omit=dev && npx playwright install chromium
CMD ["node", "login.js"]
