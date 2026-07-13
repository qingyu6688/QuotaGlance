import { readFileSync } from "node:fs";

function readJson(relativePath) {
  return JSON.parse(readFileSync(new URL(relativePath, import.meta.url), "utf8"));
}

function matchVersion(content, pattern, sourceName) {
  const version = content.match(pattern)?.[1];
  if (version === undefined) {
    throw new Error(`无法从 ${sourceName} 读取项目版本。`);
  }
  return version;
}

try {
  const packageJson = readJson("../package.json");
  const packageLock = readJson("../package-lock.json");
  const tauriConfig = readJson("../src-tauri/tauri.conf.json");
  const cargoManifest = readFileSync(new URL("../src-tauri/Cargo.toml", import.meta.url), "utf8");
  const cargoLock = readFileSync(new URL("../src-tauri/Cargo.lock", import.meta.url), "utf8");
  const quotaGlancePackage = cargoLock
    .split("[[package]]")
    .find((entry) => entry.includes('name = "quota-glance"'));

  if (quotaGlancePackage === undefined) {
    throw new Error("Cargo.lock 中缺少 quota-glance 项目包。");
  }

  const versions = {
    package: packageJson.version,
    packageLock: packageLock.version,
    packageLockRoot: packageLock.packages[""].version,
    tauri: tauriConfig.version,
    cargo: matchVersion(cargoManifest, /^version = "([^"]+)"/m, "Cargo.toml"),
    cargoLock: matchVersion(quotaGlancePackage, /version = "([^"]+)"/, "Cargo.lock"),
  };
  const expectedVersion = versions.package;
  const mismatches = Object.entries(versions).filter(([, version]) => version !== expectedVersion);

  if (mismatches.length > 0) {
    const details = mismatches.map(([source, version]) => `${source}=${version}`).join("，");
    throw new Error(`发布版本不一致：package=${expectedVersion}，${details}。`);
  }

  const refType = process.argv[2];
  const refName = process.argv[3];
  if (refType === "tag" && refName !== `v${expectedVersion}`) {
    throw new Error(`发布标签 ${refName} 与项目版本 v${expectedVersion} 不一致。`);
  }

  console.log(`发布版本一致性检查通过：${expectedVersion}`);
} catch (error) {
  const message = error instanceof Error ? error.message : "发布版本检查失败。";
  console.error(message);
  process.exitCode = 1;
}
