FROM ubuntu:24.04

# Install system deps (same as the .deb depends)
RUN apt-get update && apt-get install -y \
    libgtk-3-0 \
    libwebkit2gtk-4.1-0 \
    libgdk-pixbuf-2.0-0 \
    libjavascriptcoregtk-4.1-0 \
    libsoup-3.0-0 \
    && rm -rf /var/lib/apt/lists/*

COPY libresync_*.deb /tmp/libresync.deb

RUN dpkg -i /tmp/libresync.deb 2>&1 || true && \
    apt-get install -y -f 2>&1 && \
    rm /tmp/libresync.deb

CMD ["libresync-core", "--help"]
