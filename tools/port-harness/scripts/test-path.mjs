import { loadConfig } from "../dist/config.js";
import { resolveSourceFilePath } from "../dist/files/paths.js";
import { existsSync } from "node:fs";

const c = loadConfig();
console.log("referenceRoot:", c.referenceRoot);
for (const p of ["Spell.cpp", "src/game/Spells/Spell.cpp"]) {
  const resolved = resolveSourceFilePath(c.referenceRoot, p);
  console.log(p, "->", resolved, existsSync(resolved));
}
