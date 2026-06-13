export interface FileListEntry {
  path: string;
  name: string;
  kind: "cpp" | "h";
  size_bytes: number;
  indexed: boolean;
  symbol_count: number;
  discovered: number;
  documented: number;
  blocked: number;
  flow_count: number;
}

export interface TaskWithDetails {
  id: number;
  source_symbol_id: number;
  flow_id: number | null;
  target_rust_file: string | null;
  status: string;
  notes: string | null;
  rust_symbol_name: string | null;
  symbol_name: string;
  symbol_file: string;
  start_line: number;
  end_line: number;
  flow_name: string | null;
  risk_level: string | null;
  claim_count: number;
  fixture_count: number;
}

export interface Stats {
  status_counts: { status: string; count: number }[];
  file_progress: { file: string; total: number; documented: number }[];
  recent_jobs: PipelineJob[];
  warnings: { blocked: number; missing_docs: number };
}

export interface FlowSummary {
  id: number;
  name: string;
  description: string | null;
  source_file: string | null;
  risk_level: string;
  symbol_count: number;
}

export interface JobTarget {
  taskId: number;
  symbolName: string;
  file: string;
  state: "done" | "running" | "pending";
}

export interface PipelineJob {
  id: number;
  stage: string;
  target_ids: string;
  status: string;
  progress: number;
  total: number;
  error: string | null;
  current_item: string | null;
  log: string;
  created_at: string;
  finished_at: string | null;
  summary: string;
  stage_label: string;
  targets: JobTarget[];
  last_log_line: string | null;
  active_job_id?: number | null;
  active_job_ids?: number[];
  is_stale?: boolean;
  pause_requested?: boolean;
  display_status?: string;
}

export interface FlowAuditSummary {
  total: number;
  audited: number;
  complete: number;
  partial: number;
  missing: number;
  incorrect: number;
  reviewed: number;
  passed: number;
}

export interface TaskAuditDetail {
  task_id: number;
  audited_at: string;
  implementation_status: "complete" | "partial" | "missing" | "incorrect";
  passed: boolean;
  coverage: {
    claims_covered: number;
    claims_total: number;
    branches_covered?: number;
    branches_total?: number;
  };
  summary: string;
  issues: { severity: string; message: string; claim_ref?: string }[];
  rust_locations: { file: string; symbol: string }[];
}

export interface TaskPlanSummary {
  target_rust_file: string;
  rust_symbol_name: string;
  structs: string[];
  enums: string[];
  notes: string;
  planned_at: string | null;
}

export interface TaskPortDraft {
  rust_code: string;
  todos: string[];
  ported_at: string | null;
  file_path: string | null;
}

export interface FlowDetailResponse {
  flow: {
    name: string;
    description: string | null;
    expected_behaviour: string | null;
  };
  branches: { condition: string; behaviour: string; file: string; start_line: number }[];
  mutations: { variable_or_field: string; mutation_description: string }[];
  tasks: (TaskWithDetails & {
    audit: TaskAuditDetail | null;
    plan: TaskPlanSummary | null;
    port_draft: TaskPortDraft | null;
    next_action: "audit" | "plan" | "port" | "review" | "done";
  })[];
  audit_summary: FlowAuditSummary;
  pipeline_jobs: PipelineJob[];
}

const BASE = "/api";

async function fetchJson<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${url}`, {
    headers: { "Content-Type": "application/json" },
    ...init,
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error((err as { error: string }).error ?? res.statusText);
  }
  return res.json() as Promise<T>;
}

export const api = {
  getStats: () => fetchJson<Stats>("/stats"),

  getFiles: (params: { q?: string; kind?: string } = {}) => {
    const qs = new URLSearchParams();
    if (params.q) qs.set("q", params.q);
    if (params.kind) qs.set("kind", params.kind);
    return fetchJson<{ files: FileListEntry[]; total: number; indexed_count: number }>(
      `/files?${qs}`,
    );
  },

  indexFile: (path: string) =>
    fetchJson<{ ok: boolean; path: string; symbolsIndexed: number }>("/files/index", {
      method: "POST",
      body: JSON.stringify({ path }),
    }),

  documentFile: (path: string, status = "discovered") =>
    fetchJson<{ ok: boolean; jobIds: number[]; totalTasks: number; batches: number }>(
      "/files/document",
      { method: "POST", body: JSON.stringify({ path, status }) },
    ),

  assembleFlowsForFile: (path: string) =>
    fetchJson<{ ok: boolean; jobId: number }>("/files/assemble-flows", {
      method: "POST",
      body: JSON.stringify({ path }),
    }),

  runFilePipeline: (path: string) =>
    fetchJson<{
      ok: boolean;
      extractJobIds: number[];
      assembleJobId: number;
      totalTasks: number;
    }>("/files/pipeline", { method: "POST", body: JSON.stringify({ path }) }),

  getTasks: (params: Record<string, string | undefined> = {}) => {
    const qs = new URLSearchParams();
    for (const [k, v] of Object.entries(params)) {
      if (v) qs.set(k, v);
    }
    return fetchJson<{ tasks: TaskWithDetails[]; total: number }>(`/tasks?${qs}`);
  },

  getTask: (id: number) =>
    fetchJson<{ task: TaskWithDetails; claims: unknown[]; dependencies: unknown[] }>(
      `/tasks/${id}`,
    ),

  bulkUpdateTasks: (ids: number[], fields: Record<string, unknown>) =>
    fetchJson<{ updated: number }>("/tasks/bulk", {
      method: "PATCH",
      body: JSON.stringify({ ids, ...fields }),
    }),

  getFlows: () => fetchJson<FlowSummary[]>("/flows"),

  getFlow: (id: number) => fetchJson<FlowDetailResponse>(`/flows/${id}`),

  auditFlow: (id: number) =>
    fetchJson<{ ok: boolean; jobIds: number[]; totalTasks: number; batches: number }>(
      `/flows/${id}/audit`,
      { method: "POST" },
    ),

  planFlow: (id: number) =>
    fetchJson<{ ok: boolean; jobIds: number[]; totalTasks: number; batches: number }>(
      `/flows/${id}/plan`,
      { method: "POST" },
    ),

  portFlow: (id: number) =>
    fetchJson<{ ok: boolean; jobIds: number[]; totalTasks: number; batches: number }>(
      `/flows/${id}/port`,
      { method: "POST" },
    ),

  getSymbolSource: (id: number) =>
    fetchJson<{ symbol: unknown; snippet: string; start_line: number; end_line: number }>(
      `/symbols/${id}/source`,
    ),

  createJob: (stage: string, taskIds: number[]) =>
    fetchJson<{ jobId: number; total: number }>("/jobs", {
      method: "POST",
      body: JSON.stringify({ stage, taskIds }),
    }),

  getJobs: () => fetchJson<PipelineJob[]>("/jobs"),

  getJob: (id: number) => fetchJson<PipelineJob>(`/jobs/${id}`),

  retryJob: (id: number) =>
    fetchJson<{ ok: boolean; jobId: number }>(`/jobs/${id}/retry`, { method: "POST" }),

  continueJob: (id: number) =>
    fetchJson<{ ok: boolean; jobId: number }>(`/jobs/${id}/continue`, { method: "POST" }),

  cancelJob: (id: number) =>
    fetchJson<{ ok: boolean }>(`/jobs/${id}/cancel`, { method: "POST" }),

  abandonJob: (id: number) =>
    fetchJson<{ ok: boolean }>(`/jobs/${id}/abandon`, { method: "POST" }),

  pauseJob: (id: number) =>
    fetchJson<{ ok: boolean; immediate?: boolean; pausing?: boolean }>(`/jobs/${id}/pause`, {
      method: "POST",
    }),

  resumeJob: (id: number) =>
    fetchJson<{ ok: boolean; jobId: number }>(`/jobs/${id}/resume`, { method: "POST" }),

  deleteJob: (id: number) =>
    fetchJson<{ ok: boolean }>(`/jobs/${id}`, { method: "DELETE" }),

  listInvestigations: () =>
    fetchJson<
      {
        id: number;
        query: string;
        status: string;
        job_id: number | null;
        created_at: string;
        finished_at: string | null;
        candidate_count: number;
      }[]
    >("/discover"),

  getInvestigation: (id: number) =>
    fetchJson<{
      investigation: {
        id: number;
        query: string;
        status: string;
        seed_json: string | null;
        result_json: string | null;
        job_id: number | null;
        created_at: string;
        finished_at: string | null;
      };
      result: DiscoverResult | null;
      seed: unknown;
      job: PipelineJob | null;
    }>(`/discover/${id}`),

  startDiscovery: (query: string) =>
    fetchJson<{ ok: boolean; investigationId: number; jobId: number; seedHitCount: number }>(
      "/discover",
      { method: "POST", body: JSON.stringify({ query }) },
    ),

  discoverActions: (
    investigationId: number,
    body: { action: "index" | "extract" | "verify"; paths?: string[]; taskIds?: number[] },
  ) =>
    fetchJson<{ ok: boolean; jobIds?: number[]; results?: unknown[] }>(
      `/discover/${investigationId}/actions`,
      { method: "POST", body: JSON.stringify(body) },
    ),

  indexDirectory: (dir = "src/game") =>
    fetchJson<{ ok: boolean; dir: string; filesIndexed: number; symbolsIndexed: number }>(
      "/discover/index-dir",
      { method: "POST", body: JSON.stringify({ dir }) },
    ),
};

export interface DiscoverCandidate {
  symbol: string;
  file: string;
  relevance: "high" | "medium" | "low";
  reason: string;
  task_id?: number;
  task_status?: string;
  flow_name?: string;
  in_index: boolean;
  suggested_action: "index" | "extract" | "verify" | "inspect" | "check_rust";
}

export interface DiscoverResult {
  hypothesis: string;
  areas: { name: string; description: string }[];
  candidates: DiscoverCandidate[];
  unindexed_files: { path: string; reason: string }[];
  suggested_next_steps: string[];
}
