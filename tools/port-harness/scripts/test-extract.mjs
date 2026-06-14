import { resolve } from "node:path";
import { performance } from "node:perf_hooks";
import { extractMethodsFromCpp } from "../src/index/cppMethods.ts";

const path = resolve(
  import.meta.dirname,
  "..",
  "..",
  "..",
  "reference",
  "core",
  "src/game/Handlers/AuctionHouseHandler.cpp",
);

const t0 = performance.now();
const methods = extractMethodsFromCpp(path, []);
console.log("ms", (performance.now() - t0).toFixed(1));
console.log("count", methods.length);
console.log(methods.map((m) => m.name).join("\n"));
