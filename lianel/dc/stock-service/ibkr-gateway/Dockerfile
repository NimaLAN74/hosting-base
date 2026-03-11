# IBKR Client Portal Gateway – official zip, our conf.yaml for Docker IP allow list.
# API base URL when using this gateway: https://ibkr-gateway:5000/v1/api (set IBKR_API_BASE_URL).
# After starting, open https://<host>:5000 and log in with your IBKR account (same account as OAuth tokens).
FROM eclipse-temurin:21-jre-alpine

EXPOSE 5000

ENV PACKAGE=clientportal.gw.zip

WORKDIR /client-portal

RUN apk add --no-cache unzip curl \
  && curl -sSL -o "${PACKAGE}" "https://download2.interactivebrokers.com/portal/${PACKAGE}" \
  && unzip -q "${PACKAGE}" \
  && rm -f "${PACKAGE}"

COPY conf.yaml root/conf.yaml

# run.sh may reference bash; Alpine has ash
RUN sed -i 's|#!/bin/bash|#!/bin/sh|g' bin/run.sh 2>/dev/null || true

CMD ["/bin/sh", "bin/run.sh", "root/conf.yaml"]
