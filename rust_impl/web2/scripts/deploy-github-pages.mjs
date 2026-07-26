import { cp, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";

const webDirectory = fileURLToPath(new URL("../", import.meta.url));
const repositoryDirectory = fileURLToPath(new URL("../../../", import.meta.url));
const publishDirectory = await mkdtemp(join(tmpdir(), "snipe-hunt-pages-"));
const dryRun = process.argv.includes("--dry-run");

function run(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: options.cwd ?? webDirectory,
      stdio: "inherit",
    });
    child.on("error", reject);
    child.on("exit", (code, signal) => {
      if (code === 0) {
        resolve();
      } else {
        reject(
          new Error(
            `${command} ${args.join(" ")} failed${signal ? ` with signal ${signal}` : ` with exit code ${code}`}`,
          ),
        );
      }
    });
  });
}

function capture(command, args, cwd) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd, stdio: ["ignore", "pipe", "inherit"] });
    let output = "";
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      output += chunk;
    });
    child.on("error", reject);
    child.on("exit", (code) => {
      if (code === 0) resolve(output.trim());
      else reject(new Error(`${command} ${args.join(" ")} failed with exit code ${code}`));
    });
  });
}

try {
  await run("npm", ["run", "build:github-pages"]);
  await cp(join(webDirectory, "dist/client"), publishDirectory, { recursive: true });
  await writeFile(join(publishDirectory, ".nojekyll"), "");

  const origin = await capture("git", ["remote", "get-url", "origin"], repositoryDirectory);
  await run("git", ["init"], { cwd: publishDirectory });
  await run("git", ["add", "--all"], { cwd: publishDirectory });
  await run(
    "git",
    [
      "-c",
      "user.name=Snipe Hunt deploy",
      "-c",
      "user.email=deploy@snipe-hunt.invalid",
      "commit",
      "-m",
      "Deploy Snipe Hunt",
    ],
    { cwd: publishDirectory },
  );
  if (dryRun) {
    console.log("Dry run complete; skipped the push to GitHub Pages.");
  } else {
    await run("git", ["push", "--force", origin, "HEAD:gh-pages"], { cwd: publishDirectory });
    console.log("Published to https://kylejlin.github.io/snipe-hunt/");
  }
} finally {
  await rm(publishDirectory, { recursive: true, force: true });
}
