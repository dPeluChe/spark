#!/usr/bin/env node

/**
 * SPARK postinstall script
 * Downloads the correct binary for the current platform from GitHub releases.
 */

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const https = require("https");

const VERSION = require("./package.json").version;
const REPO = "dPeluChe/labs-spark";
const BIN_DIR = path.join(__dirname, "bin");
const BIN_PATH = path.join(BIN_DIR, "spark");

function getPlatformKey() {
  const platform = process.platform;
  const arch = process.arch;

  const map = {
    "darwin-arm64": "spark-macos-arm64",
    "darwin-x64": "spark-macos-x64",
    "linux-x64": "spark-linux-x64",
  };

  const key = `${platform}-${arch}`;
  const artifact = map[key];

  if (!artifact) {
    console.error(`  Unsupported platform: ${key}`);
    console.error(`  Supported: ${Object.keys(map).join(", ")}`);
    console.error(`  You can build from source: cargo install --git https://github.com/${REPO}`);
    process.exit(1);
  }

  return artifact;
}

function download(url) {
  return new Promise((resolve, reject) => {
    const follow = (url, redirects = 0) => {
      if (redirects > 5) return reject(new Error("Too many redirects"));

      https.get(url, { headers: { "User-Agent": "spark-installer" } }, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          return follow(res.headers.location, redirects + 1);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`HTTP ${res.statusCode}: ${url}`));
        }
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      }).on("error", reject);
    };
    follow(url);
  });
}

async function main() {
  // Skip if binary already exists and is correct version
  if (fs.existsSync(BIN_PATH)) {
    try {
      const version = execSync(`"${BIN_PATH}" --version`, { encoding: "utf-8" }).trim();
      if (version.includes(VERSION)) {
        console.log(`  spark v${VERSION} already installed`);
        return;
      }
    } catch (_) {
      // Binary exists but can't run — re-download
    }
  }

  const artifact = getPlatformKey();
  const tag = `v${VERSION}`;
  const url = `https://github.com/${REPO}/releases/download/${tag}/${artifact}`;

  console.log(`  Downloading spark ${tag} for ${process.platform}-${process.arch}...`);

  try {
    const binary = await download(url);

    fs.mkdirSync(BIN_DIR, { recursive: true });
    fs.writeFileSync(BIN_PATH, binary);
    fs.chmodSync(BIN_PATH, 0o755);

    console.log(`  Installed spark to ${BIN_PATH}`);
    console.log(`  Run 'spark init' to complete setup.`);
  } catch (err) {
    console.error(`  Failed to download: ${err.message}`);
    console.error(`  URL: ${url}`);
    console.error(`\n  Alternative: build from source`);
    console.error(`  cargo install --git https://github.com/${REPO}`);
    process.exit(1);
  }
}

main();
