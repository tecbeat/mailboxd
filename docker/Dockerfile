FROM ubuntu:24.04
ARG CRATE_VERSION
ARG TARGETARCH

LABEL maintainer="tecbeat"
LABEL version="${CRATE_VERSION}"
LABEL description="mailboxd — self-hosted email archiving server"
LABEL org.opencontainers.image.source="https://git.teccave.de/tecbeat/mailboxd"
LABEL org.opencontainers.image.licenses="AGPL-3.0-or-later"

WORKDIR /opt/mailboxd

COPY ${TARGETARCH}/mailboxd-server /opt/mailboxd/mailboxd-server
COPY ${TARGETARCH}/LICENSE /opt/mailboxd/
COPY ${TARGETARCH}/NOTICE /opt/mailboxd/

COPY ${TARGETARCH}/mailboxd-cli /usr/local/bin/mailboxd-cli
COPY ${TARGETARCH}/mailboxd-admin /usr/local/bin/mailboxd-admin

RUN chmod +x /opt/mailboxd/mailboxd-server \
    && chmod +x /usr/local/bin/mailboxd-cli \
    && chmod +x /usr/local/bin/mailboxd-admin

RUN apt update && apt install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*
RUN mkdir -p /data

EXPOSE 15630

WORKDIR /data

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD curl -fs http://localhost:15630/api/status || exit 1

CMD ["/opt/mailboxd/mailboxd-server"]
