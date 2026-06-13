import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative, resolve } from "node:path";

const SKIP_DIRS = new Set(["node_modules", "target", ".git", "dist"]);
const MAX_FILE_BYTES = 120_000;
const MAX_TOTAL_CHARS = 24_000;

export interface RustLookupResult {
  snippets: string;
  searchNotes: string;
  locations: { file: string; line?: number }[];
}

function walkRustFiles(dir: string, root: string, out: string[]): void {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }

  for (const entry of entries) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (!SKIP_DIRS.has(entry.name)) walkRustFiles(full, root, out);
      continue;
    }
    if (entry.name.endsWith(".rs")) {
      out.push(relative(root, full).replace(/\\/g, "/"));
    }
  }
}

function readBounded(path: string): string | null {
  try {
    if (!existsSync(path)) return null;
    if (statSync(path).size > MAX_FILE_BYTES) return null;
    return readFileSync(path, "utf-8");
  } catch {
    return null;
  }
}

function extractSnippetAroundMatch(source: string, needle: string, context = 40): string | null {
  const idx = source.indexOf(needle);
  if (idx < 0) return null;
  const startLine = source.slice(0, idx).split("\n").length;
  const lines = source.split("\n");
  const from = Math.max(0, startLine - 1 - context);
  const to = Math.min(lines.length, startLine - 1 + context);
  return lines
    .slice(from, to)
    .map((line, i) => `${from + i + 1}: ${line}`)
    .join("\n");
}

function searchNameCandidates(cppSymbolName: string, rustSymbolName?: string | null): string[] {
  const names = new Set<string>();
  if (rustSymbolName?.trim()) names.add(rustSymbolName.trim());

  const method = cppSymbolName.split("::").pop() ?? cppSymbolName;
  names.add(method);
  names.add(
    method.replace(/([a-z0-9])([A-Z])/g, "$1_$2").replace(/([A-Z]+)([A-Z][a-z])/g, "$1_$2").toLowerCase(),
  );

  if (method.startsWith("Get")) {
    names.add(method.slice(3));
    names.add(method.slice(3).toLowerCase());
  }

  return [...names].filter((n) => n.length >= 3);
}

export function findRustImplementation(
  rustRoot: string,
  opts: {
    cppSymbolName: string;
    targetFile?: string | null;
    rustSymbolName?: string | null;
  },
): RustLookupResult {
  const locations: { file: string; line?: number }[] = [];
  const parts: string[] = [];
  const notes: string[] = [];
  let totalChars = 0;

  const addSnippet = (relPath: string, label: string, body: string) => {
    const chunk = `### ${label} (${relPath})\n\`\`\`rust\n${body}\n\`\`\`\n`;
    if (totalChars + chunk.length > MAX_TOTAL_CHARS) return false;
    parts.push(chunk);
    totalChars += chunk.length;
    locations.push({ file: relPath });
    return true;
  };

  if (opts.targetFile?.trim()) {
    const rel = opts.targetFile.replace(/\\/g, "/").replace(/^\.\//, "");
    const full = resolve(rustRoot, rel);
    const source = readBounded(full);
    if (source) {
      const names = searchNameCandidates(opts.cppSymbolName, opts.rustSymbolName);
      let added = false;
      for (const name of names) {
        const snip = extractSnippetAroundMatch(source, name, 50);
        if (snip && addSnippet(rel, `match: ${name}`, snip)) {
          notes.push(`Found "${name}" in planned target ${rel}`);
          added = true;
          break;
        }
      }
      if (!added && addSnippet(rel, "full file (truncated search)", source.slice(0, 8000))) {
        notes.push(`Read planned target ${rel} (no exact symbol match)`);
      }
    } else {
      notes.push(`Planned target not found: ${rel}`);
    }
  }

  const names = searchNameCandidates(opts.cppSymbolName, opts.rustSymbolName);
  const rustFiles: string[] = [];
  walkRustFiles(resolve(rustRoot, "src"), rustRoot, rustFiles);

  for (const name of names) {
    for (const rel of rustFiles) {
      if (parts.length >= 6) break;
      const full = resolve(rustRoot, rel);
      const source = readBounded(full);
      if (!source || !source.includes(name)) continue;
      const snip = extractSnippetAroundMatch(source, name, 35);
      if (!snip) continue;
      const label = `match: ${name}`;
      if (parts.some((p) => p.includes(rel) && p.includes(name))) continue;
      if (addSnippet(rel, label, snip)) {
        notes.push(`Found "${name}" in ${rel}`);
      }
    }
  }

  if (parts.length === 0) {
    notes.push(`No Rust matches for candidates: ${names.join(", ")}`);
  }

  return {
    snippets: parts.join("\n") || "(no Rust implementation found in src/)",
    searchNotes: notes.join("; "),
    locations,
  };
}
