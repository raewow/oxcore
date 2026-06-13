import { readFileSync } from "node:fs";
import { basename } from "node:path";

export interface CppMethodSymbol {
  file: string;
  name: string;
  kind: "method";
  startLine: number;
  endLine: number;
}

// C++ method definition at line start: optional return type, then Class::Method(
const CPP_METHOD_DEF_PATTERN =
  /^\s*(?:template\s*<[^>]*>\s*)?(?:[\w:<>,\s*&]+\s+)+(\w+)::(~?\w+)\s*\(/;

function findMethodEndLine(lines: string[], startIdx: number): number {
  let braceDepth = 0;
  let foundOpen = false;

  for (let i = startIdx; i < lines.length; i++) {
    for (const ch of lines[i]!) {
      if (ch === "{") {
        braceDepth++;
        foundOpen = true;
      } else if (ch === "}") {
        braceDepth--;
      }
    }
    if (foundOpen && braceDepth === 0) return i + 1;
    if (i === startIdx && lines[i]!.includes(")") && !lines[i]!.includes("{")) {
      if (i + 1 < lines.length && !lines[i + 1]!.includes("{")) {
        return i + 1;
      }
    }
  }

  return Math.min(startIdx + 100, lines.length);
}

export function extractMethodsFromCpp(
  cppPath: string,
  excludePatterns: string[],
): CppMethodSymbol[] {
  const source = readFileSync(cppPath, "utf-8");
  const file = basename(cppPath);
  const lines = source.split("\n");
  const symbols: CppMethodSymbol[] = [];
  const seen = new Set<string>();

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!;
    const match = line.match(CPP_METHOD_DEF_PATTERN);
    if (!match) continue;

    const className = match[1]!;
    const methodName = match[2]!;
    if (methodName.startsWith("Effect")) continue;

    let excluded = false;
    for (const pattern of excludePatterns) {
      if (pattern.endsWith("*") && methodName.startsWith(pattern.slice(0, -1))) {
        excluded = true;
        break;
      }
      if (methodName === pattern) {
        excluded = true;
        break;
      }
    }
    if (excluded) continue;

    const fullName = `${className}::${methodName}`;
    if (seen.has(fullName)) continue;
    seen.add(fullName);

    const endLine = findMethodEndLine(lines, i);
    symbols.push({
      file,
      name: fullName,
      kind: "method",
      startLine: i + 1,
      endLine,
    });
  }

  return symbols;
}
