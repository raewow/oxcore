import type Database from "better-sqlite3";
import { getFeatureTasks } from "./features.js";

/**
 * Feature coverage = "is every detail mapped?"
 *
 * Walks the call graph outward from a feature's symbols. Each callee is one of:
 *   - in_feature:  resolves to an indexed symbol already assigned to this feature
 *   - outside:     resolves to an indexed symbol that is NOT in the feature (boundary
 *                  should probably expand to pull it in)
 *   - unindexed:   does not resolve to any indexed symbol — a candidate gap. Most are
 *                  external/STL/locals; the port-worthy ones are surfaced by frequency
 *                  so a human/agent can triage them.
 *
 * Names are matched by short name (segment after the last "::") because call edges store
 * unqualified callee names while symbols store qualified ones.
 */

export interface CoverageGapCallee {
  callee: string;
  call_count: number;
  example_callers: string[];
}

export interface CoverageOutsideSymbol {
  symbol: string;
  file: string;
  call_count: number;
}

export interface FeatureCoverage {
  feature: string;
  feature_symbol_count: number;
  task_status: { status: string; count: number }[];
  distinct_callees: number;
  resolved_in_feature: number;
  resolved_outside_feature: number;
  unindexed: number;
  mapped_pct: number;
  /** Indexed symbols reachable from the feature but not yet assigned to it. */
  pull_into_feature: CoverageOutsideSymbol[];
  /** Unindexed callees ranked by call frequency — the triage worklist. */
  top_gaps: CoverageGapCallee[];
}

function shortName(name: string): string {
  return name.split("::").pop()!.split(":")[0];
}

/**
 * Callees that are never port targets: STL, language builtins, and lowercase tokens
 * that are almost always locals/params/variables rather than functions. Filtering these
 * keeps the call-graph closure honest instead of matching on short-name collisions.
 */
function isNoiseCallee(calleeName: string): boolean {
  if (calleeName.startsWith("std::") || calleeName.startsWith("boost::")) return true;
  const short = shortName(calleeName);
  // bare lowercase identifiers with no namespace are overwhelmingly locals (target, searcher)
  if (!calleeName.includes("::") && /^[a-z_]/.test(calleeName)) return true;
  return /^(sizeof|static_cast|reinterpret_cast|dynamic_cast|const_cast|move|make_pair|min|max|abs|swap)$/.test(
    short,
  );
}

export function getFeatureCoverage(
  db: Database.Database,
  featureId: number,
  featureName: string,
  opts: { gapLimit?: number; pullLimit?: number } = {},
): FeatureCoverage {
  const gapLimit = opts.gapLimit ?? 40;
  const pullLimit = opts.pullLimit ?? 40;

  // Index of every known symbol by short name -> [{name,file,id}]
  const allSymbols = db
    .prepare("SELECT id, name, file FROM code_symbol")
    .all() as { id: number; name: string; file: string }[];
  const byShort = new Map<string, { id: number; name: string; file: string }[]>();
  for (const s of allSymbols) {
    const key = shortName(s.name);
    const list = byShort.get(key);
    if (list) list.push(s);
    else byShort.set(key, [s]);
  }

  const featureTasks = getFeatureTasks(db, featureId);
  const featureSymbolIds = new Set(featureTasks.map((t) => t.source_symbol_id));
  const featureShortNames = new Set(featureTasks.map((t) => shortName(t.symbol_name)));

  const statusCounts = new Map<string, number>();
  for (const t of featureTasks) {
    statusCounts.set(t.status, (statusCounts.get(t.status) ?? 0) + 1);
  }

  // All call edges originating from feature symbols.
  let edges: { callee_name: string; caller: string }[] = [];
  if (featureSymbolIds.size > 0) {
    const ph = [...featureSymbolIds].map(() => "?").join(",");
    edges = db
      .prepare(
        `SELECT sc.callee_name, cs.name AS caller
         FROM symbol_call sc
         JOIN code_symbol cs ON cs.id = sc.caller_id
         WHERE sc.caller_id IN (${ph})`,
      )
      .all(...[...featureSymbolIds]) as { callee_name: string; caller: string }[];
  }

  const gaps = new Map<string, { count: number; callers: Set<string> }>();
  const outside = new Map<string, { file: string; count: number }>();
  let inFeature = 0;
  let outsideCount = 0;
  let unindexed = 0;
  const distinct = new Set<string>();

  for (const e of edges) {
    if (isNoiseCallee(e.callee_name)) continue;
    const short = shortName(e.callee_name);
    distinct.add(short);
    const matches = byShort.get(short);
    if (!matches) {
      unindexed++;
      const g = gaps.get(e.callee_name) ?? { count: 0, callers: new Set<string>() };
      g.count++;
      if (g.callers.size < 4) g.callers.add(e.caller);
      gaps.set(e.callee_name, g);
    } else if (featureShortNames.has(short)) {
      inFeature++;
    } else {
      outsideCount++;
      const m = matches[0];
      const o = outside.get(m.name) ?? { file: m.file, count: 0 };
      o.count++;
      outside.set(m.name, o);
    }
  }

  const total = inFeature + outsideCount + unindexed;
  const mapped = inFeature + outsideCount;

  return {
    feature: featureName,
    feature_symbol_count: featureTasks.length,
    task_status: [...statusCounts.entries()]
      .map(([status, count]) => ({ status, count }))
      .sort((a, b) => b.count - a.count),
    distinct_callees: distinct.size,
    resolved_in_feature: inFeature,
    resolved_outside_feature: outsideCount,
    unindexed,
    mapped_pct: total === 0 ? 100 : Math.round((mapped / total) * 100),
    pull_into_feature: [...outside.entries()]
      .map(([symbol, v]) => ({ symbol, file: v.file, call_count: v.count }))
      .sort((a, b) => b.call_count - a.call_count)
      .slice(0, pullLimit),
    top_gaps: [...gaps.entries()]
      .map(([callee, v]) => ({
        callee,
        call_count: v.count,
        example_callers: [...v.callers],
      }))
      .sort((a, b) => b.call_count - a.call_count)
      .slice(0, gapLimit),
  };
}
