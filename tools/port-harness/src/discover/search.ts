import type Database from "better-sqlite3";
import { getCallGraphNeighbors } from "../db/repositories/codeSymbol.js";

const STOPWORDS = new Set([
  "a", "an", "the", "is", "are", "was", "were", "be", "been", "being",
  "have", "has", "had", "do", "does", "did", "will", "would", "could",
  "should", "may", "might", "must", "shall", "can", "need", "there",
  "where", "when", "what", "which", "who", "how", "why", "if", "but",
  "and", "or", "not", "no", "yes", "i", "my", "me", "we", "our", "you",
  "your", "it", "its", "this", "that", "these", "those", "to", "for",
  "of", "in", "on", "at", "by", "with", "from", "as", "into", "through",
  "during", "before", "after", "about", "bug", "where",
]);

export interface DiscoverSeedHit {
  symbol_id: number;
  symbol: string;
  file: string;
  source: "symbol" | "claim" | "flow" | "branch" | "task_note" | "call_graph";
  match_text: string;
  score: number;
  task_id?: number;
  task_status?: string;
  flow_name?: string;
}

export interface FilePortStats {
  file: string;
  total: number;
  documented: number;
  verified: number;
}

export function tokenizeQuery(query: string): string[] {
  return query
    .toLowerCase()
    .split(/[^\w]+/)
    .filter((w) => w.length > 2 && !STOPWORDS.has(w));
}

function scoreText(text: string, keywords: string[], weight: number): number {
  const lower = text.toLowerCase();
  let score = 0;
  for (const kw of keywords) {
    if (lower.includes(kw)) score += weight;
  }
  return score;
}

function addHit(
  hits: Map<string, DiscoverSeedHit>,
  hit: Omit<DiscoverSeedHit, "score"> & { score?: number },
  keywords: string[],
  weight: number,
): void {
  const key = `${hit.symbol_id}:${hit.source}`;
  const score = hit.score ?? scoreText(hit.match_text, keywords, weight);
  if (score <= 0) return;

  const existing = hits.get(key);
  if (!existing || score > existing.score) {
    hits.set(key, { ...hit, score: (existing?.score ?? 0) + score });
  } else if (existing) {
    existing.score += score * 0.5;
  }
}

export function searchHarness(
  db: Database.Database,
  query: string,
  limit = 40,
): DiscoverSeedHit[] {
  const keywords = tokenizeQuery(query);
  if (!keywords.length) {
    return [];
  }

  const hits = new Map<string, DiscoverSeedHit>();
  const symbolParams: string[] = [];
  for (const kw of keywords) {
    const pattern = `%${kw}%`;
    symbolParams.push(pattern, pattern, pattern);
  }

  const symbolRows = db
    .prepare(
      `SELECT cs.id AS symbol_id, cs.name AS symbol, cs.file,
              COALESCE(cs.summary, '') AS match_text,
              mt.id AS task_id, mt.status AS task_status,
              bf.name AS flow_name
       FROM code_symbol cs
       LEFT JOIN migration_task mt ON mt.source_symbol_id = cs.id
       LEFT JOIN business_flow bf ON bf.id = mt.flow_id
       WHERE ${keywords.map(() => "LOWER(cs.name) LIKE ? OR LOWER(cs.file) LIKE ? OR LOWER(COALESCE(cs.summary, '')) LIKE ?").join(" OR ")}`,
    )
    .all(...symbolParams) as {
    symbol_id: number;
    symbol: string;
    file: string;
    match_text: string;
    task_id: number | null;
    task_status: string | null;
    flow_name: string | null;
  }[];

  for (const row of symbolRows) {
    addHit(
      hits,
      {
        symbol_id: row.symbol_id,
        symbol: row.symbol,
        file: row.file,
        source: "symbol",
        match_text: row.match_text || row.symbol,
        task_id: row.task_id ?? undefined,
        task_status: row.task_status ?? undefined,
        flow_name: row.flow_name ?? undefined,
      },
      keywords,
      3,
    );
  }

  const claimRows = db
    .prepare(
      `SELECT cs.id AS symbol_id, cs.name AS symbol, cs.file,
              bc.claim_text AS match_text,
              mt.id AS task_id, mt.status AS task_status,
              bf.name AS flow_name
       FROM behaviour_claim bc
       JOIN code_symbol cs ON cs.id = bc.symbol_id
       LEFT JOIN migration_task mt ON mt.source_symbol_id = cs.id
       LEFT JOIN business_flow bf ON bf.id = mt.flow_id
       WHERE ${keywords.map(() => "LOWER(bc.claim_text) LIKE ?").join(" OR ")}`,
    )
    .all(...keywords.map((kw) => `%${kw}%`)) as typeof symbolRows;

  for (const row of claimRows) {
    addHit(
      hits,
      {
        symbol_id: row.symbol_id,
        symbol: row.symbol,
        file: row.file,
        source: "claim",
        match_text: row.match_text,
        task_id: row.task_id ?? undefined,
        task_status: row.task_status ?? undefined,
        flow_name: row.flow_name ?? undefined,
      },
      keywords,
      5,
    );
  }

  const flowRows = db
    .prepare(
      `SELECT bf.id, bf.name, bf.description, bf.expected_behaviour, bf.source_file
       FROM business_flow bf
       WHERE ${keywords.map(() => "LOWER(bf.name) LIKE ? OR LOWER(COALESCE(bf.description, '')) LIKE ? OR LOWER(COALESCE(bf.expected_behaviour, '')) LIKE ?").join(" OR ")}`,
    )
    .all(...keywords.flatMap((kw) => [`%${kw}%`, `%${kw}%`, `%${kw}%`])) as {
    id: number;
    name: string;
    description: string | null;
    expected_behaviour: string | null;
    source_file: string | null;
  }[];

  for (const flow of flowRows) {
    const matchText = [flow.name, flow.description, flow.expected_behaviour]
      .filter(Boolean)
      .join(" — ");
    const taskRows = db
      .prepare(
        `SELECT cs.id AS symbol_id, cs.name AS symbol, cs.file,
                mt.id AS task_id, mt.status AS task_status
         FROM migration_task mt
         JOIN code_symbol cs ON cs.id = mt.source_symbol_id
         WHERE mt.flow_id = ?
         LIMIT 3`,
      )
      .all(flow.id) as {
      symbol_id: number;
      symbol: string;
      file: string;
      task_id: number;
      task_status: string;
    }[];

    for (const row of taskRows) {
      addHit(
        hits,
        {
          symbol_id: row.symbol_id,
          symbol: row.symbol,
          file: row.file,
          source: "flow",
          match_text: matchText,
          task_id: row.task_id,
          task_status: row.task_status,
          flow_name: flow.name,
        },
        keywords,
        4,
      );
    }
  }

  const branchRows = db
    .prepare(
      `SELECT lb.condition, lb.behaviour, lb.file,
              cs.id AS symbol_id, cs.name AS symbol,
              mt.id AS task_id, mt.status AS task_status,
              bf.name AS flow_name
       FROM logic_branch lb
       JOIN business_flow bf ON bf.id = lb.flow_id
       LEFT JOIN code_symbol cs ON cs.file = lb.file AND cs.start_line <= lb.start_line AND cs.end_line >= lb.end_line
       LEFT JOIN migration_task mt ON mt.source_symbol_id = cs.id
       WHERE ${keywords.map(() => "LOWER(lb.condition) LIKE ? OR LOWER(lb.behaviour) LIKE ?").join(" OR ")}`,
    )
    .all(...keywords.flatMap((kw) => [`%${kw}%`, `%${kw}%`])) as {
    condition: string;
    behaviour: string;
    file: string;
    symbol_id: number | null;
    symbol: string | null;
    task_id: number | null;
    task_status: string | null;
    flow_name: string | null;
  }[];

  for (const row of branchRows) {
    if (!row.symbol_id || !row.symbol) continue;
    addHit(
      hits,
      {
        symbol_id: row.symbol_id,
        symbol: row.symbol,
        file: row.file,
        source: "branch",
        match_text: `${row.condition} → ${row.behaviour}`,
        task_id: row.task_id ?? undefined,
        task_status: row.task_status ?? undefined,
        flow_name: row.flow_name ?? undefined,
      },
      keywords,
      4,
    );
  }

  const noteRows = db
    .prepare(
      `SELECT cs.id AS symbol_id, cs.name AS symbol, cs.file,
              mt.notes AS match_text, mt.id AS task_id, mt.status AS task_status
       FROM migration_task mt
       JOIN code_symbol cs ON cs.id = mt.source_symbol_id
       WHERE mt.notes IS NOT NULL AND (${keywords.map(() => "LOWER(mt.notes) LIKE ?").join(" OR ")})`,
    )
    .all(...keywords.map((kw) => `%${kw}%`)) as typeof symbolRows;

  for (const row of noteRows) {
    addHit(
      hits,
      {
        symbol_id: row.symbol_id,
        symbol: row.symbol,
        file: row.file,
        source: "task_note",
        match_text: row.match_text,
        task_id: row.task_id ?? undefined,
        task_status: row.task_status ?? undefined,
      },
      keywords,
      3,
    );
  }

  const topHits = [...hits.values()]
    .sort((a, b) => b.score - a.score)
    .slice(0, limit);

  const topSymbolIds = topHits.slice(0, 15).map((h) => h.symbol_id);
  const graphNeighbors = getCallGraphNeighbors(db, topSymbolIds, 20);

  for (const neighbor of graphNeighbors) {
    const taskRow = db
      .prepare(
        `SELECT mt.id AS task_id, mt.status AS task_status, bf.name AS flow_name
         FROM migration_task mt
         LEFT JOIN business_flow bf ON bf.id = mt.flow_id
         WHERE mt.source_symbol_id = ?`,
      )
      .get(neighbor.symbol_id) as {
      task_id: number;
      task_status: string;
      flow_name: string | null;
    } | undefined;

    addHit(
      hits,
      {
        symbol_id: neighbor.symbol_id,
        symbol: neighbor.symbol,
        file: neighbor.file,
        source: "call_graph",
        match_text: `${neighbor.relation} of ${neighbor.related_symbol}`,
        task_id: taskRow?.task_id,
        task_status: taskRow?.task_status,
        flow_name: taskRow?.flow_name ?? undefined,
        score: 2,
      },
      keywords,
      2,
    );
  }

  return [...hits.values()]
    .sort((a, b) => b.score - a.score)
    .slice(0, limit + 20);
}

export function getMatchedFileStats(
  db: Database.Database,
  hits: DiscoverSeedHit[],
): FilePortStats[] {
  const files = [...new Set(hits.map((h) => h.file))];
  const stats: FilePortStats[] = [];

  for (const file of files) {
    const row = db
      .prepare(
        `SELECT COUNT(*) AS total,
                SUM(CASE WHEN mt.status NOT IN ('discovered', 'blocked') THEN 1 ELSE 0 END) AS documented,
                SUM(CASE WHEN mt.status IN ('verified', 'reviewed', 'done') THEN 1 ELSE 0 END) AS verified
         FROM code_symbol cs
         JOIN migration_task mt ON mt.source_symbol_id = cs.id
         WHERE cs.file = ? OR cs.file LIKE ?`,
      )
      .get(file, `%/${file.split("/").pop()}`) as {
      total: number;
      documented: number;
      verified: number;
    };

    stats.push({
      file,
      total: row?.total ?? 0,
      documented: row?.documented ?? 0,
      verified: row?.verified ?? 0,
    });
  }

  return stats;
}
