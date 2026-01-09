# Build stage
FROM node:18-alpine as build

WORKDIR /app

# Copy package files
COPY package.json package-lock.json* ./

# Install dependencies (including dev dependencies for build)
RUN npm ci --legacy-peer-deps || npm install --legacy-peer-deps

COPY public ./public
COPY src ./src

# Build the React app
RUN npm run build

# Production stage
FROM node:18-alpine

WORKDIR /app

# Copy package.json and install production dependencies only
COPY package.json package-lock.json* ./
RUN npm ci --only=production --legacy-peer-deps || npm install --only=production --legacy-peer-deps

# Copy built React app and server
COPY --from=build /app/build ./build
COPY server.js ./

EXPOSE 80

CMD ["node", "server.js"]
