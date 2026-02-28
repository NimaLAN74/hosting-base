# Build stage
FROM node:20-alpine as build

WORKDIR /app

# Copy package files
COPY package.json package-lock.json* ./

# Install dependencies (including dev dependencies for build)
RUN npm ci --legacy-peer-deps || npm install --legacy-peer-deps

COPY public ./public
COPY src ./src

# Build arguments for environment variables (REACT_APP_ prefix required)
ARG REACT_APP_KEYCLOAK_URL
ARG REACT_APP_KEYCLOAK_REALM
ARG REACT_APP_KEYCLOAK_CLIENT_ID
ARG REACT_APP_GRAFANA_URL
ARG REACT_APP_AIRFLOW_URL
ARG REACT_APP_FRONTEND_URL
ARG REACT_APP_DOMAIN_MAIN
ARG REACT_APP_DOMAIN_ALT
ARG REACT_APP_DOMAIN_AUTH
ARG REACT_APP_DOMAIN_MONITORING
ARG REACT_APP_DOMAIN_AIRFLOW
ARG REACT_APP_OAUTH2_PROXY_CLIENT_ID

# Set environment variables for build
ENV REACT_APP_KEYCLOAK_URL=$REACT_APP_KEYCLOAK_URL
ENV REACT_APP_KEYCLOAK_REALM=$REACT_APP_KEYCLOAK_REALM
ENV REACT_APP_KEYCLOAK_CLIENT_ID=$REACT_APP_KEYCLOAK_CLIENT_ID
ENV REACT_APP_GRAFANA_URL=$REACT_APP_GRAFANA_URL
ENV REACT_APP_AIRFLOW_URL=$REACT_APP_AIRFLOW_URL
ENV REACT_APP_FRONTEND_URL=$REACT_APP_FRONTEND_URL
ENV REACT_APP_DOMAIN_MAIN=$REACT_APP_DOMAIN_MAIN
ENV REACT_APP_DOMAIN_ALT=$REACT_APP_DOMAIN_ALT
ENV REACT_APP_DOMAIN_AUTH=$REACT_APP_DOMAIN_AUTH
ENV REACT_APP_DOMAIN_MONITORING=$REACT_APP_DOMAIN_MONITORING
ENV REACT_APP_DOMAIN_AIRFLOW=$REACT_APP_DOMAIN_AIRFLOW
ENV REACT_APP_OAUTH2_PROXY_CLIENT_ID=$REACT_APP_OAUTH2_PROXY_CLIENT_ID

# Build the React app
RUN npm run build

# Production stage
FROM node:20-alpine

WORKDIR /app

# Copy package.json and install production dependencies only
COPY package.json package-lock.json* ./
RUN npm ci --only=production --legacy-peer-deps || npm install --only=production --legacy-peer-deps

# Copy built React app and server
COPY --from=build /app/build ./build
COPY server.js ./

EXPOSE 80

CMD ["node", "server.js"]
