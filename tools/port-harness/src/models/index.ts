import { z } from "zod";

export const SymbolKind = z.enum([
  "function",
  "class",
  "struct",
  "enum",
  "method",
  "macro",
  "chunk",
]);
export type SymbolKind = z.infer<typeof SymbolKind>;

export const ClaimCategory = z.enum([
  "input",
  "output",
  "branch",
  "side_effect",
  "assumption",
  "danger",
  "unknown",
]);
export type ClaimCategory = z.infer<typeof ClaimCategory>;

export const TaskStatus = z.enum([
  "discovered",
  "documented",
  "fixture_defined",
  "rust_planned",
  "rust_ported",
  "rust_compiled",
  "verified",
  "reviewed",
  "done",
  "blocked",
]);
export type TaskStatus = z.infer<typeof TaskStatus>;

export const RiskLevel = z.enum(["low", "medium", "high", "critical"]);
export type RiskLevel = z.infer<typeof RiskLevel>;

export const DependencyType = z.enum([
  "file",
  "db",
  "network",
  "global",
  "config",
  "memory",
]);
export type DependencyType = z.infer<typeof DependencyType>;

export const PipelineStage = z.enum([
  "index",
  "extract",
  "assemble-flows",
  "fixtures",
  "plan-rust",
  "port",
  "verify",
  "discover",
  "audit-rust",
]);
export type PipelineStage = z.infer<typeof PipelineStage>;

export const JobStatus = z.enum([
  "queued",
  "running",
  "paused",
  "done",
  "failed",
  "cancelled",
]);
export type JobStatus = z.infer<typeof JobStatus>;

export const CodeSymbolSchema = z.object({
  id: z.number(),
  file: z.string(),
  name: z.string(),
  kind: SymbolKind,
  start_line: z.number(),
  end_line: z.number(),
  parent_symbol_id: z.number().nullable(),
  summary: z.string().nullable(),
  created_at: z.string(),
});
export type CodeSymbol = z.infer<typeof CodeSymbolSchema>;

export const BehaviourClaimSchema = z.object({
  id: z.number().optional(),
  symbol_id: z.number(),
  category: ClaimCategory,
  claim_text: z.string().min(1),
  file: z.string().min(1),
  start_line: z.number().int().positive(),
  end_line: z.number().int().positive(),
  confidence: z.enum(["high", "medium", "low"]).default("high"),
});
export type BehaviourClaim = z.infer<typeof BehaviourClaimSchema>;

export const ExtractOutputSchema = z.object({
  summary: z.string(),
  claims: z.array(
    z.object({
      category: ClaimCategory,
      claim_text: z.string(),
      file: z.string(),
      start_line: z.number(),
      end_line: z.number(),
      confidence: z.enum(["high", "medium", "low"]).optional(),
    }),
  ),
  dependencies: z
    .array(
      z.object({
        type: DependencyType,
        description: z.string(),
        file: z.string().optional(),
        start_line: z.number().optional(),
      }),
    )
    .optional(),
});
export type ExtractOutput = z.infer<typeof ExtractOutputSchema>;

export const FlowOutputSchema = z.object({
  flows: z.array(
    z.object({
      name: z.string(),
      description: z.string(),
      entry_symbols: z.array(z.string()),
      expected_behaviour: z.string(),
      risk_level: RiskLevel,
      branches: z.array(
        z.object({
          condition: z.string(),
          behaviour: z.string(),
          file: z.string(),
          start_line: z.number(),
          end_line: z.number(),
        }),
      ),
      mutations: z.array(
        z.object({
          variable_or_field: z.string(),
          mutation_description: z.string(),
          file: z.string(),
          start_line: z.number(),
          end_line: z.number(),
        }),
      ),
    }),
  ),
});
export type FlowOutput = z.infer<typeof FlowOutputSchema>;

export const FixtureOutputSchema = z.object({
  fixtures: z.array(
    z.object({
      name: z.string(),
      description: z.string(),
      input: z.record(z.unknown()),
      expected: z.record(z.unknown()),
      covers_branches: z.array(z.string()),
    }),
  ),
});
export type FixtureOutput = z.infer<typeof FixtureOutputSchema>;

export const RustPlanOutputSchema = z.object({
  target_rust_file: z.string(),
  rust_symbol_name: z.string(),
  structs: z.array(z.string()).optional(),
  enums: z.array(z.string()).optional(),
  notes: z.string().optional(),
});
export type RustPlanOutput = z.infer<typeof RustPlanOutputSchema>;

export const PortOutputSchema = z.object({
  rust_code: z.string(),
  todos: z.array(z.string()).optional(),
  port_comments: z.array(
    z.object({
      file: z.string(),
      start_line: z.number(),
      end_line: z.number(),
    }),
  ),
});
export type PortOutput = z.infer<typeof PortOutputSchema>;

export const VerifyOutputSchema = z.object({
  passed: z.boolean(),
  issues: z.array(
    z.object({
      severity: z.enum(["error", "warning", "info"]),
      category: z.string(),
      message: z.string(),
    }),
  ),
});
export type VerifyOutput = z.infer<typeof VerifyOutputSchema>;

export const AuditOutputSchema = z.object({
  implementation_status: z.enum(["complete", "partial", "missing", "incorrect"]),
  rust_locations: z.array(
    z.object({
      file: z.string(),
      symbol: z.string(),
      start_line: z.number().optional(),
      end_line: z.number().optional(),
    }),
  ),
  coverage: z.object({
    claims_covered: z.number(),
    claims_total: z.number(),
    branches_covered: z.number().optional(),
    branches_total: z.number().optional(),
  }),
  passed: z.boolean(),
  issues: z.array(
    z.object({
      severity: z.enum(["error", "warning", "info"]),
      category: z.string(),
      message: z.string(),
      claim_ref: z.string().optional(),
    }),
  ),
  summary: z.string(),
});
export type AuditOutput = z.infer<typeof AuditOutputSchema>;

export const DiscoverRelevance = z.enum(["high", "medium", "low"]);
export type DiscoverRelevance = z.infer<typeof DiscoverRelevance>;

export const DiscoverSuggestedAction = z.enum([
  "index",
  "extract",
  "verify",
  "inspect",
  "check_rust",
]);
export type DiscoverSuggestedAction = z.infer<typeof DiscoverSuggestedAction>;

export const DiscoverOutputSchema = z.object({
  hypothesis: z.string(),
  areas: z.array(
    z.object({
      name: z.string(),
      description: z.string(),
    }),
  ),
  candidates: z.array(
    z.object({
      symbol: z.string(),
      file: z.string(),
      relevance: DiscoverRelevance,
      reason: z.string(),
      task_id: z.number().optional(),
      task_status: z.string().optional(),
      flow_name: z.string().optional(),
      in_index: z.boolean(),
      suggested_action: DiscoverSuggestedAction,
    }),
  ),
  unindexed_files: z.array(
    z.object({
      path: z.string(),
      reason: z.string(),
    }),
  ),
  suggested_next_steps: z.array(z.string()),
});
export type DiscoverOutput = z.infer<typeof DiscoverOutputSchema>;

export const InvestigationStatus = z.enum(["running", "done", "failed"]);
export type InvestigationStatus = z.infer<typeof InvestigationStatus>;

export const MigrationTaskSchema = z.object({
  id: z.number(),
  source_symbol_id: z.number(),
  flow_id: z.number().nullable(),
  target_rust_file: z.string().nullable(),
  status: TaskStatus,
  notes: z.string().nullable(),
  rust_symbol_name: z.string().nullable(),
  updated_at: z.string(),
  created_at: z.string(),
});
export type MigrationTask = z.infer<typeof MigrationTaskSchema>;

export interface TaskWithDetails extends MigrationTask {
  symbol_name: string;
  symbol_file: string;
  start_line: number;
  end_line: number;
  flow_name: string | null;
  risk_level: string | null;
  claim_count: number;
  fixture_count: number;
}

export const TASK_STATUS_ORDER: TaskStatus[] = [
  "discovered",
  "documented",
  "fixture_defined",
  "rust_planned",
  "rust_ported",
  "rust_compiled",
  "verified",
  "reviewed",
  "done",
  "blocked",
];

export function canTransitionToPort(status: TaskStatus): boolean {
  return status === "documented" || status === "fixture_defined" || status === "rust_planned";
}

export function matchesExcludePattern(name: string, patterns: string[]): boolean {
  for (const pattern of patterns) {
    if (pattern.endsWith("*")) {
      const prefix = pattern.slice(0, -1);
      if (name.startsWith(prefix)) return true;
    } else if (name === pattern) {
      return true;
    }
  }
  return false;
}
