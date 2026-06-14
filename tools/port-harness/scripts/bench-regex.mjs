import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { performance } from "node:perf_hooks";

const p = resolve(
  import.meta.dirname,
  "..",
  "..",
  "..",
  "reference",
  "core",
  "src/game/Handlers/AuctionHouseHandler.cpp",
);
const lines = readFileSync(p, "utf-8").split("\n");

const good =
  /^\s*(?:template\s*<[^>]*>\s*)?(?:\S+\s+)*(\w+)::(~?\w+)\s*\(/;

function bench(name, re) {
  const t0 = performance.now();
  let hits = 0;
  for (const line of lines) {
    if (!line.includes("::") || !line.includes("(")) continue;
    if (re.test(line)) hits++;
  }
  console.log(name, (performance.now() - t0).toFixed(1) + "ms", "hits", hits);
}

bench("good", good);
