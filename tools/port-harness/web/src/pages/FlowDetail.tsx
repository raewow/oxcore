import { Fragment, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api, type FlowDetailResponse } from "../api/client";
import { StatusBadge } from "../components/StatusBadge";

const NEXT_ACTION_LABELS: Record<string, string> = {
  audit: "Run audit",
  plan: "Plan Rust",
  port: "Port symbol",
  review: "Mark reviewed",
  done: "Done",
};

function AuditStatusBadge({ status }: { status: string | null }) {
  if (!status) {
    return <span className="audit-badge audit-none">not audited</span>;
  }
  return <span className={`audit-badge audit-${status}`}>{status}</span>;
}

function JobStatusBadge({ status }: { status: string }) {
  return <span className={`status-badge status-${status}`}>{status}</span>;
}

type DetailTab = "audit" | "plan" | "draft";

function SymbolDetailPanel({
  task,
  defaultTab,
}: {
  task: FlowDetailResponse["tasks"][number];
  defaultTab: DetailTab;
}) {
  const [tab, setTab] = useState<DetailTab>(defaultTab);
  const tabs: { id: DetailTab; label: string; available: boolean }[] = [
    { id: "audit", label: "Audit", available: !!task.audit },
    { id: "plan", label: "Plan", available: !!task.plan },
    { id: "draft", label: "Draft", available: !!task.port_draft },
  ];
  const activeTabs = tabs.filter((t) => t.available);

  return (
    <div className="symbol-detail-panel">
      <div className="symbol-detail-tabs">
        {activeTabs.map((t) => (
          <button
            key={t.id}
            type="button"
            className={`symbol-detail-tab${tab === t.id ? " active" : ""}`}
            onClick={() => setTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === "audit" && task.audit && (
        <div>
          <p style={{ marginBottom: "0.5rem" }}>
            <strong>Summary:</strong> {task.audit.summary}
          </p>
          {task.audit.rust_locations.length > 0 && (
            <p style={{ marginBottom: "0.5rem", fontSize: "0.85rem" }}>
              <strong>Rust:</strong>{" "}
              {task.audit.rust_locations
                .map((l) => `${l.symbol} in ${l.file}`)
                .join("; ")}
            </p>
          )}
          {task.audit.issues.length > 0 && (
            <>
              <strong>Issues:</strong>
              <ul className="audit-issues">
                {task.audit.issues.map((issue, i) => (
                  <li key={i} className={`audit-issue-${issue.severity}`}>
                    [{issue.severity}] {issue.message}
                  </li>
                ))}
              </ul>
            </>
          )}
          <p style={{ fontSize: "0.8rem", color: "#94a3b8", marginTop: "0.5rem" }}>
            Full report:{" "}
            <code>tools/port-harness/docs/audits/{task.symbol_name.replace(/::/g, "_")}.md</code>
          </p>
        </div>
      )}

      {tab === "plan" && task.plan && (
        <div>
          <p style={{ marginBottom: "0.5rem" }}>
            <strong>Target file:</strong> <code>{task.plan.target_rust_file || "—"}</code>
          </p>
          <p style={{ marginBottom: "0.5rem" }}>
            <strong>Rust symbol:</strong> <code>{task.plan.rust_symbol_name || "—"}</code>
          </p>
          {task.plan.structs.length > 0 && (
            <p style={{ marginBottom: "0.5rem" }}>
              <strong>Structs:</strong> {task.plan.structs.join(", ")}
            </p>
          )}
          {task.plan.enums.length > 0 && (
            <p style={{ marginBottom: "0.5rem" }}>
              <strong>Enums:</strong> {task.plan.enums.join(", ")}
            </p>
          )}
          {task.plan.notes && (
            <p style={{ marginBottom: "0.5rem", whiteSpace: "pre-wrap" }}>
              <strong>Notes:</strong> {task.plan.notes}
            </p>
          )}
          {task.plan.planned_at && (
            <p style={{ fontSize: "0.8rem", color: "#94a3b8" }}>
              Planned: {new Date(task.plan.planned_at).toLocaleString()}
            </p>
          )}
          <p style={{ fontSize: "0.8rem", color: "#94a3b8", marginTop: "0.5rem" }}>
            Full plan:{" "}
            <code>tools/port-harness/docs/plans/{task.symbol_name.replace(/::/g, "_")}.md</code>
          </p>
        </div>
      )}

      {tab === "draft" && task.port_draft && (
        <div>
          {task.port_draft.todos.length > 0 && (
            <>
              <strong>TODOs:</strong>
              <ul className="audit-issues">
                {task.port_draft.todos.map((todo, i) => (
                  <li key={i}>{todo}</li>
                ))}
              </ul>
            </>
          )}
          <pre className="artifact-code">{task.port_draft.rust_code}</pre>
          {task.port_draft.ported_at && (
            <p style={{ fontSize: "0.8rem", color: "#94a3b8", marginTop: "0.5rem" }}>
              Ported: {new Date(task.port_draft.ported_at).toLocaleString()}
            </p>
          )}
          {task.port_draft.file_path && (
            <p style={{ fontSize: "0.8rem", color: "#94a3b8", marginTop: "0.25rem" }}>
              File: <code>{task.port_draft.file_path}</code>
            </p>
          )}
        </div>
      )}
    </div>
  );
}

function defaultDetailTab(task: FlowDetailResponse["tasks"][number]): DetailTab {
  if (task.audit) return "audit";
  if (task.plan) return "plan";
  return "draft";
}

function hasSymbolDetail(task: FlowDetailResponse["tasks"][number]): boolean {
  return !!(task.audit || task.plan || task.port_draft);
}

export function FlowDetail() {
  const { id } = useParams<{ id: string }>();
  const flowId = parseInt(id ?? "0", 10);
  const queryClient = useQueryClient();
  const [expandedTaskId, setExpandedTaskId] = useState<number | null>(null);

  const { data, isLoading } = useQuery({
    queryKey: ["flow", flowId],
    queryFn: () => api.getFlow(flowId),
    enabled: flowId > 0,
    refetchInterval: (query) => {
      const jobs = query.state.data?.pipeline_jobs ?? [];
      const active = jobs.some(
        (j) =>
          j.status === "running" ||
          j.status === "queued" ||
          j.pause_requested ||
          j.display_status === "pausing",
      );
      return active ? 2000 : false;
    },
  });

  const invalidateFlow = () => {
    queryClient.invalidateQueries({ queryKey: ["flow", flowId] });
    queryClient.invalidateQueries({ queryKey: ["jobs"] });
    queryClient.invalidateQueries({ queryKey: ["tasks"] });
  };

  const auditMutation = useMutation({
    mutationFn: () => api.auditFlow(flowId),
    onSuccess: invalidateFlow,
  });

  const planMutation = useMutation({
    mutationFn: () => api.planFlow(flowId),
    onSuccess: invalidateFlow,
  });

  const portMutation = useMutation({
    mutationFn: () => api.portFlow(flowId),
    onSuccess: invalidateFlow,
  });

  const portTaskMutation = useMutation({
    mutationFn: (taskId: number) => api.createJob("port", [taskId]),
    onSuccess: invalidateFlow,
  });

  if (isLoading) return <div>Loading...</div>;
  if (!data) return <div>Flow not found</div>;

  const flow = data as FlowDetailResponse;
  const summary = flow.audit_summary;
  const needsPlan = flow.tasks.filter((t) => t.next_action === "plan").length;
  const needsAudit = flow.tasks.filter((t) => t.next_action === "audit").length;
  const needsPort = flow.tasks.filter((t) => t.next_action === "port").length;
  const activeJobs = flow.pipeline_jobs.filter(
    (j) => j.status === "running" || j.status === "queued" || j.display_status === "pausing",
  );

  return (
    <div>
      <div className="page-header">
        <h2>{flow.flow.name}</h2>
        <p style={{ color: "#94a3b8" }}>{flow.flow.description}</p>
        <div className="action-buttons" style={{ marginTop: "0.75rem" }}>
          <button
            type="button"
            className="btn"
            disabled={auditMutation.isPending || flow.tasks.length === 0}
            onClick={() => {
              if (
                window.confirm(
                  `Audit Rust implementation for ${flow.tasks.length} symbol(s)?`,
                )
              ) {
                auditMutation.mutate();
              }
            }}
          >
            {auditMutation.isPending ? "Queueing…" : "Audit implementation"}
          </button>
          <button
            type="button"
            className="btn btn-secondary"
            disabled={planMutation.isPending || needsPlan === 0}
            onClick={() => {
              if (
                window.confirm(
                  `Plan Rust for ${needsPlan} symbol(s) that need work?`,
                )
              ) {
                planMutation.mutate();
              }
            }}
          >
            {planMutation.isPending ? "Queueing…" : `Plan changes (${needsPlan})`}
          </button>
          <button
            type="button"
            className="btn btn-secondary"
            disabled={portMutation.isPending || needsPort === 0}
            onClick={() => {
              if (
                window.confirm(
                  `Generate Rust drafts for ${needsPort} planned symbol(s)? Output goes to docs/ports/.`,
                )
              ) {
                portMutation.mutate();
              }
            }}
          >
            {portMutation.isPending ? "Queueing…" : `Port drafts (${needsPort})`}
          </button>
          <Link to="/jobs" className="btn btn-secondary">
            Jobs
          </Link>
        </div>
      </div>

      <div className="summary-cards">
        <div className="summary-card">
          <span className="summary-label">Audited</span>
          <span className="summary-value">
            {summary.audited}/{summary.total}
          </span>
        </div>
        <div className="summary-card">
          <span className="summary-label">Missing</span>
          <span className="summary-value summary-missing">{summary.missing}</span>
        </div>
        <div className="summary-card">
          <span className="summary-label">Partial</span>
          <span className="summary-value summary-partial">{summary.partial}</span>
        </div>
        <div className="summary-card">
          <span className="summary-label">Complete</span>
          <span className="summary-value summary-complete">{summary.complete}</span>
        </div>
        <div className="summary-card">
          <span className="summary-label">Passed</span>
          <span className="summary-value summary-complete">{summary.passed}</span>
        </div>
      </div>

      {activeJobs.length > 0 && (
        <div className="info-banner" style={{ marginBottom: "1rem" }}>
          <strong>Pipeline jobs running:</strong>{" "}
          {activeJobs.map((j) => (
            <span key={j.id} style={{ marginRight: "1rem" }}>
              <Link to="/jobs">#{j.id}</Link> {j.stage_label} {j.progress}/{j.total}{" "}
              <JobStatusBadge status={j.display_status ?? j.status} />
              {j.current_item ? ` — ${j.current_item}` : ""}
            </span>
          ))}
        </div>
      )}

      <div className="next-steps-panel">
        <h3>What&apos;s next?</h3>
        <ol>
          {needsAudit > 0 && (
            <li>
              <strong>Audit</strong> — {needsAudit} symbol(s) not audited yet. Click{" "}
              <em>Audit implementation</em>.
            </li>
          )}
          {needsPlan > 0 && (
            <li>
              <strong>Plan</strong> — {needsPlan} symbol(s) are missing or partial. Click{" "}
              <em>Plan changes</em> to decide target Rust files and types.
            </li>
          )}
          {needsPort > 0 && (
            <li>
              <strong>Port</strong> — {needsPort} symbol(s) are planned. Click{" "}
              <em>Port drafts</em> to generate Rust stubs in <code>docs/ports/</code>.
            </li>
          )}
          {summary.complete > 0 && (
            <li>
              <strong>Review</strong> — {summary.complete} symbol(s) look complete in Rust. Audit
              marks them <code>reviewed</code> when passed.
            </li>
          )}
          {needsAudit === 0 &&
            needsPlan === 0 &&
            needsPort === 0 &&
            summary.audited === summary.total && (
              <li>All symbols audited — nothing left to do on this flow.</li>
            )}
        </ol>
      </div>

      {flow.flow.expected_behaviour && (
        <>
          <h3 style={{ marginBottom: "0.5rem" }}>Expected Behaviour</h3>
          <p style={{ marginBottom: "1.5rem" }}>{flow.flow.expected_behaviour}</p>
        </>
      )}

      {flow.pipeline_jobs.length > 0 && (
        <>
          <h3 style={{ marginBottom: "0.75rem" }}>Pipeline jobs</h3>
          <table style={{ marginBottom: "1.5rem" }}>
            <thead>
              <tr>
                <th>Job</th>
                <th>Stage</th>
                <th>Status</th>
                <th>Progress</th>
                <th>Current</th>
              </tr>
            </thead>
            <tbody>
              {flow.pipeline_jobs.map((j) => (
                <tr key={j.id}>
                  <td>
                    <Link to="/jobs">#{j.id}</Link>
                  </td>
                  <td style={{ fontSize: "0.85rem" }}>{j.stage_label}</td>
                  <td>
                    <JobStatusBadge status={j.display_status ?? j.status} />
                  </td>
                  <td>
                    {j.progress}/{j.total}
                  </td>
                  <td style={{ fontSize: "0.85rem" }}>{j.current_item ?? j.last_log_line ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}

      <h3 style={{ marginBottom: "0.75rem" }}>Symbols ({flow.tasks.length})</h3>
      <table>
        <thead>
          <tr>
            <th>Symbol</th>
            <th>Task</th>
            <th>Audit</th>
            <th>Plan</th>
            <th>Draft</th>
            <th>Coverage</th>
            <th>Next</th>
            <th>Action</th>
            <th>Rust target</th>
          </tr>
        </thead>
        <tbody>
          {flow.tasks.map((t) => (
            <Fragment key={t.id}>
              <tr
                key={t.id}
                className={expandedTaskId === t.id ? "row-selected" : undefined}
                onClick={() =>
                  hasSymbolDetail(t) &&
                  setExpandedTaskId(expandedTaskId === t.id ? null : t.id)
                }
                style={{ cursor: hasSymbolDetail(t) ? "pointer" : undefined }}
              >
                <td>
                  <Link to={`/symbols/${t.source_symbol_id}`} onClick={(e) => e.stopPropagation()}>
                    {t.symbol_name}
                  </Link>
                </td>
                <td>
                  <StatusBadge status={t.status} />
                </td>
                <td>
                  <AuditStatusBadge status={t.audit?.implementation_status ?? null} />
                  {t.audit?.passed && (
                    <span className="audit-badge audit-passed" style={{ marginLeft: 6 }}>
                      passed
                    </span>
                  )}
                </td>
                <td style={{ fontSize: "0.85rem" }}>
                  {t.plan ? (
                    <span className="artifact-badge artifact-planned">planned</span>
                  ) : (
                    "—"
                  )}
                </td>
                <td style={{ fontSize: "0.85rem" }}>
                  {t.port_draft ? (
                    <span className="artifact-badge artifact-drafted">draft</span>
                  ) : (
                    "—"
                  )}
                </td>
                <td style={{ fontSize: "0.85rem" }}>
                  {t.audit
                    ? `${t.audit.coverage.claims_covered}/${t.audit.coverage.claims_total}`
                    : "—"}
                </td>
                <td style={{ fontSize: "0.85rem" }}>{NEXT_ACTION_LABELS[t.next_action]}</td>
                <td onClick={(e) => e.stopPropagation()}>
                  {t.next_action === "port" && (
                    <button
                      type="button"
                      className="btn btn-sm"
                      disabled={portTaskMutation.isPending}
                      onClick={() => {
                        if (window.confirm(`Port ${t.symbol_name}?`)) {
                          portTaskMutation.mutate(t.id);
                        }
                      }}
                    >
                      Port
                    </button>
                  )}
                  {t.next_action === "plan" && (
                    <span style={{ color: "#94a3b8", fontSize: "0.8rem" }}>use Plan changes</span>
                  )}
                  {t.next_action === "audit" && (
                    <span style={{ color: "#94a3b8", fontSize: "0.8rem" }}>use Audit</span>
                  )}
                  {t.next_action === "done" && (
                    <span style={{ color: "#86efac", fontSize: "0.8rem" }}>✓</span>
                  )}
                </td>
                <td style={{ fontSize: "0.8rem" }}>
                  {t.plan?.target_rust_file ?? t.audit?.rust_locations[0]?.file ?? t.target_rust_file ?? "—"}
                </td>
              </tr>
              {expandedTaskId === t.id && hasSymbolDetail(t) && (
                <tr key={`${t.id}-detail`}>
                  <td colSpan={9} className="audit-detail-cell">
                    <SymbolDetailPanel task={t} defaultTab={defaultDetailTab(t)} />
                  </td>
                </tr>
              )}
            </Fragment>
          ))}
        </tbody>
      </table>

      {flow.branches.length > 0 && (
        <>
          <h3 style={{ margin: "1.5rem 0 0.75rem" }}>Branches</h3>
          <table>
            <thead>
              <tr>
                <th>Condition</th>
                <th>Behaviour</th>
                <th>Source</th>
              </tr>
            </thead>
            <tbody>
              {flow.branches.map((b, i) => (
                <tr key={i}>
                  <td>{b.condition}</td>
                  <td>{b.behaviour}</td>
                  <td>
                    {b.file}:{b.start_line}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}

      {flow.mutations.length > 0 && (
        <>
          <h3 style={{ margin: "1.5rem 0 0.75rem" }}>State Mutations</h3>
          <table>
            <thead>
              <tr>
                <th>Variable</th>
                <th>Description</th>
              </tr>
            </thead>
            <tbody>
              {flow.mutations.map((m, i) => (
                <tr key={i}>
                  <td>{m.variable_or_field}</td>
                  <td>{m.mutation_description}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}
    </div>
  );
}
