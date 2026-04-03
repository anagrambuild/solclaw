# SolClaw Agent Container
# Runs Claude Agent SDK in isolated Linux VM with browser automation

FROM node:22-slim

# Install system dependencies for Chromium
RUN apt-get update && apt-get install -y \
    chromium \
    fonts-liberation \
    fonts-noto-color-emoji \
    libgbm1 \
    libnss3 \
    libatk-bridge2.0-0 \
    libgtk-3-0 \
    libx11-xcb1 \
    libxcomposite1 \
    libxdamage1 \
    libxrandr2 \
    libasound2 \
    libpangocairo-1.0-0 \
    libcups2 \
    libdrm2 \
    libxshmfence1 \
    curl \
    git \
    && rm -rf /var/lib/apt/lists/*

# Set Chromium path for agent-browser
ENV AGENT_BROWSER_EXECUTABLE_PATH=/usr/bin/chromium
ENV PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH=/usr/bin/chromium

# Install agent-browser, claude-code, and tsx globally (before NODE_OPTIONS is set)
RUN npm install -g agent-browser @anthropic-ai/claude-code tsx

# Create app directory
WORKDIR /app

# Copy package files first for better caching
COPY agent-runner/package*.json ./

# Install dependencies
RUN npm install

# Copy source code
COPY agent-runner/ ./

# Copy Solana transaction auto-logging preload script
COPY solana-tx-preload.cjs /app/solana-tx-preload.cjs

# Build TypeScript
RUN npm run build

# Symlink global tsx into local node_modules so the entrypoint can resolve it from /app/
RUN ln -s $(npm root -g)/tsx node_modules/tsx

# ESM interop for all node processes (safe, no tsx dependency)
ENV NODE_OPTIONS="--experimental-require-module --require /app/solana-tx-preload.cjs"

# Create workspace and IPC directories
RUN mkdir -p /workspace/group /workspace/global /workspace/extra /data/ipc/messages /data/ipc/tasks /data/ipc/input /data/ipc/transactions

# Backward-compat symlink: legacy scripts may still reference /workspace/ipc
RUN ln -sf /data/ipc /workspace/ipc

# Create entrypoint script
# Secrets are passed via stdin JSON — temp file is deleted immediately after Node reads it
# Follow-up messages arrive via IPC files in /data/ipc/input/
RUN printf '#!/bin/bash\nset -e\ncd /app && npx tsc --outDir /tmp/dist 2>&1 >&2\nln -s /app/node_modules /tmp/dist/node_modules\nchmod -R a-w /tmp/dist\ncat > /tmp/input.json\nnode --import tsx /tmp/dist/index.js < /tmp/input.json\n' > /app/entrypoint.sh && chmod +x /app/entrypoint.sh

# Set ownership to node user (non-root) for writable directories
RUN chown -R node:node /workspace /data && chmod 777 /home/node

# Switch to non-root user (required for --dangerously-skip-permissions)
USER node

# Set working directory to group workspace
WORKDIR /workspace/group

# Entry point reads JSON from stdin, outputs JSON to stdout
ENTRYPOINT ["/app/entrypoint.sh"]
