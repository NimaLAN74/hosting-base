# Stock Monitoring UI – build and serve (no local run; pipeline only).
FROM node:18-alpine AS build

WORKDIR /app
COPY package.json package-lock.json* ./
RUN npm ci --legacy-peer-deps || npm install --legacy-peer-deps

COPY public ./public
COPY src ./src
RUN npm run build

# Production: serve static build with Node.
FROM node:18-alpine
WORKDIR /app
COPY package.json package-lock.json* ./
RUN npm ci --only=production --legacy-peer-deps || npm install --only=production --legacy-peer-deps
COPY --from=build /app/build ./build
COPY server.js ./
EXPOSE 80
CMD ["node", "server.js"]
