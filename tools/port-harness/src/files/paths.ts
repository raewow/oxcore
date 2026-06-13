import { existsSync } from "node:fs";
import { basename, resolve } from "node:path";
import { scanReferenceFiles } from "./scanner.js";

/** Normalize to forward-slash relative path (no leading slash). */
export function normalizeRelPath(p: string): string {
  return p.replace(/\\/g, "/").replace(/^\.\//, "");
}

export interface ResolvePathHint {
  /** e.g. Unit::Update — used to disambiguate basename-only stored paths */
  symbolName?: string;
}

function pickBestMatch(
  matches: { path: string; name: string }[],
  normalized: string,
  hint?: ResolvePathHint,
): { path: string; name: string } | undefined {
  if (matches.length === 0) return undefined;
  if (matches.length === 1) return matches[0];

  const exact = matches.find((m) => m.path === normalized);
  if (exact) return exact;

  if (hint?.symbolName) {
    const className = hint.symbolName.split("::")[0];
    if (className) {
      const byClassDir = matches.find((m) =>
        m.path.toLowerCase().includes(`/${className.toLowerCase()}/`),
      );
      if (byClassDir) return byClassDir;
    }
  }

  // Prefer deeper paths (more specific) over shallow ones
  return [...matches].sort((a, b) => b.path.length - a.path.length)[0];
}

/**
 * Resolve a stored symbol file path to an absolute path on disk.
 * Handles legacy DB rows that only stored the basename (e.g. "Unit.cpp").
 */
export function resolveSourceFilePath(
  referenceRoot: string,
  storedFile: string,
  hint?: ResolvePathHint,
): string {
  const normalized = normalizeRelPath(storedFile);
  const direct = resolve(referenceRoot, normalized);
  if (existsSync(direct)) return direct;

  const base = basename(normalized);
  const scanned = scanReferenceFiles(referenceRoot);
  const matches = scanned.filter((f) => f.path === normalized || f.name === base);
  const best = pickBestMatch(matches, normalized, hint);

  if (best) {
    const resolved = resolve(referenceRoot, best.path);
    if (existsSync(resolved)) return resolved;
  }

  throw new Error(
    `Source file not found for "${storedFile}" under ${referenceRoot}. ` +
      `Re-index from the Files page using the full path (e.g. src/game/Entities/Unit.cpp).`,
  );
}

export function filePathMatches(storedFile: string, query: string): boolean {
  const stored = normalizeRelPath(storedFile);
  const q = normalizeRelPath(query);
  if (stored === q) return true;
  if (stored.endsWith("/" + q)) return true;
  if (q.endsWith("/" + stored)) return true;
  return basename(stored) === basename(q);
}

/** Upgrade legacy basename-only rows to full relative paths in SQLite. */
export function resolveFullPathForStored(
  referenceRoot: string,
  storedFile: string,
  hint?: ResolvePathHint,
): string {
  const normalized = normalizeRelPath(storedFile);
  if (normalized.includes("/")) return normalized;

  const scanned = scanReferenceFiles(referenceRoot);
  const matches = scanned.filter((f) => f.name === normalized);
  const best = pickBestMatch(matches, normalized, hint);
  return best?.path ?? normalized;
}
