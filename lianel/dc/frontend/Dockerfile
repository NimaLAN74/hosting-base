# Build stage
FROM node:18-alpine as build

WORKDIR /app

COPY package.json ./
RUN npm install

COPY public ./public
COPY src ./src

RUN npm run build

# Production stage
FROM node:18-alpine

WORKDIR /app

# Copy package.json and install production dependencies
COPY package.json ./
RUN npm install --production

# Copy built React app and server
COPY --from=build /app/build ./build
COPY server.js ./

EXPOSE 80

CMD ["node", "server.js"]
