import { spawn } from "node:child_process";
import net from "node:net";
import { fileURLToPath } from "node:url";

const DEFAULT_HOST = "127.0.0.1";
const DEFAULT_PORT = 1420;
const MAX_PORT = 65535;

const rootDir = fileURLToPath(new URL("..", import.meta.url));

function parsePort(value, fallback) {
  if (!value) {
    return fallback;
  }

  const port = Number.parseInt(value, 10);
  if (!Number.isInteger(port) || port < 0 || port > MAX_PORT) {
    throw new Error(`Invalid port: ${value}`);
  }

  return port;
}

function normalizeHost(value) {
  const host = value || DEFAULT_HOST;
  if (!/^[\w.:-]+$/.test(host)) {
    throw new Error(`Invalid host: ${host}`);
  }

  return host;
}

function canListen(host, port) {
  return new Promise((resolve, reject) => {
    const server = net.createServer();

    server.once("error", (error) => {
      if (error.code === "EADDRINUSE" || error.code === "EACCES") {
        resolve(false);
        return;
      }

      reject(error);
    });

    server.once("listening", () => {
      server.close(() => resolve(true));
    });

    server.listen(port, host);
  });
}

async function findAvailablePort(host, startPort) {
  for (let port = startPort; port <= MAX_PORT; port += 1) {
    if (await canListen(host, port)) {
      return port;
    }
  }

  throw new Error(`No available port found from ${startPort} to ${MAX_PORT}.`);
}

const host = normalizeHost(process.env.TAURI_DEV_HOST ?? process.env.DEV_HOST);
const requestedPort = parsePort(process.env.DEV_PORT ?? process.env.PORT, DEFAULT_PORT);
const devUrlHost = host === "0.0.0.0" ? "127.0.0.1" : host;
const port = await findAvailablePort(host, requestedPort);
const devUrl = `http://${devUrlHost}:${port}`;

const tauriConfig = {
  build: {
    devUrl,
    beforeDevCommand: `pnpm exec vite --host ${host} --port ${port} --strictPort`,
  },
};

let isShuttingDown = false;
let child;

async function shutdown(code = 0, signal) {
  if (isShuttingDown) {
    return;
  }

  isShuttingDown = true;

  if (child && !child.killed) {
    child.kill(signal ?? "SIGTERM");
  }

  if (signal) {
    process.kill(process.pid, signal);
    return;
  }

  process.exit(code);
}

process.once("SIGINT", () => {
  void shutdown(0, "SIGINT");
});
process.once("SIGTERM", () => {
  void shutdown(0, "SIGTERM");
});

console.log(`Starting Tauri dev server on ${devUrl}`);

child = spawn(
  "pnpm",
  ["--filter", "desktop", "exec", "tauri", "dev", "--config", JSON.stringify(tauriConfig)],
  {
    cwd: rootDir,
    env: {
      ...process.env,
      DEV_HOST: host,
      DEV_PORT: String(port),
      DEV_STRICT_PORT: "true",
    },
    stdio: "inherit",
  },
);

child.on("exit", (code, signal) => {
  void shutdown(code ?? 0, signal ?? undefined);
});
