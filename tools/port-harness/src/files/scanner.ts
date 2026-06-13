import { readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const SKIP_DIRS = new Set(["node_modules", ".git", "build", "cmake-build-debug"]);

export interface ReferenceFile {
  path: string;
  name: string;
  kind: "cpp" | "h";
  size_bytes: number;
}

export function scanReferenceFiles(referenceRoot: string): ReferenceFile[] {
  const files: ReferenceFile[] = [];

  function walk(dir: string): void {
    let entries;
    try {
      entries = readdirSync(dir, { withFileTypes: true });
    } catch {
      return;
    }

    for (const entry of entries) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) {
        if (!SKIP_DIRS.has(entry.name)) walk(full);
        continue;
      }

      if (entry.name.endsWith(".cpp")) {
        files.push({
          path: relative(referenceRoot, full).replace(/\\/g, "/"),
          name: entry.name,
          kind: "cpp",
          size_bytes: statSync(full).size,
        });
      } else if (entry.name.endsWith(".h")) {
        files.push({
          path: relative(referenceRoot, full).replace(/\\/g, "/"),
          name: entry.name,
          kind: "h",
          size_bytes: statSync(full).size,
        });
      }
    }
  }

  walk(referenceRoot);
  return files.sort((a, b) => a.path.localeCompare(b.path));
}

/** Return .cpp files under a directory prefix (relative to reference root). */
export function scanReferenceCppInDir(
  referenceRoot: string,
  dirPrefix: string,
): ReferenceFile[] {
  const normalized = dirPrefix.replace(/\\/g, "/").replace(/\/$/, "");
  return scanReferenceFiles(referenceRoot).filter(
    (f) =>
      f.kind === "cpp" &&
      (!normalized || f.path === normalized || f.path.startsWith(`${normalized}/`)),
  );
}
