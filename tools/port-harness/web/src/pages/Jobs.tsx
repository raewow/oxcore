import { useEffect, useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api, type PipelineJob } from "../api/client";

const STAGE_DESCRIPTIONS: Record<string, string> = {
  extract: "Documents C++ symbol behaviour via the Cursor agent (one symbol at a time).",
  "assemble-flows": "Groups documented symbols into business flows for the file.",
  "plan-rust": "Drafts a Rust module plan for each symbol.",
  port: "Generates a Rust port draft for each symbol.",
  verify: "Reviews the port against documented behaviour claims.",
  "audit-rust": "Checks existing Rust in src/ against documented C++ behaviour.",
};

function elapsedSince(iso: string): string {
  const normalized = iso.includes("T") ? iso : `${iso.replace(" ", "T")}Z`;
  const ms = Date.now() - new Date(normalized).getTime();
  if (Number.isNaN(ms) || ms < 0) return "";
  const sec = Math.floor(ms / 1000);
  if (sec < 60) return `${sec}s`;
  const min = Math.floor(sec / 60);
  return `${min}m ${sec % 60}s`;
}

function currentLabel(job: PipelineJob): string {
  if (job.is_stale) return "stuck — no worker";
  if (job.pause_requested || job.display_status === "pausing") {
    return "finishing current symbol…";
  }
  if (job.status === "paused") {
    return `paused at ${job.progress}/${job.total}`;
  }
  if (job.status !== "running" && job.status !== "queued") return "";
  if (job.current_item) return job.current_item;
  if (job.last_log_line) return job.last_log_line.replace(/^\[\d{2}:\d{2}:\d{2}\]\s*/, "");
  return "…";
}

function normalizeJob(job: PipelineJob): PipelineJob {
  return {
    ...job,
    summary: job.summary ?? job.stage,
    stage_label: job.stage_label ?? job.stage,
    targets: job.targets ?? [],
    last_log_line: job.last_log_line ?? null,
    log: job.log ?? "",
  };
}

function displayStatus(job: PipelineJob): string {
  return job.display_status ?? (job.is_stale ? "stale" : job.status);
}

function statusBadgeClass(job: PipelineJob): string {
  const status = displayStatus(job);
  return `status-badge status-${status}`;
}

function canRetry(job: PipelineJob): boolean {
  return ["done", "failed", "cancelled"].includes(job.status);
}

function canContinue(job: PipelineJob): boolean {
  return (
    ["failed", "cancelled"].includes(job.status) &&
    job.progress < job.total &&
    job.stage !== "assemble-flows"
  );
}

function canAbandon(job: PipelineJob): boolean {
  return job.status === "running" && Boolean(job.is_stale);
}

function canPause(job: PipelineJob): boolean {
  if (job.is_stale || job.pause_requested) return false;
  if (job.status === "queued") return true;
  return job.status === "running" && (job.active_job_ids?.includes(job.id) ?? job.active_job_id === job.id);
}

function canResume(job: PipelineJob): boolean {
  return job.status === "paused" && job.progress < job.total;
}

function canCancel(job: PipelineJob): boolean {
  return job.status === "queued";
}

function canRemove(job: PipelineJob): boolean {
  return ["done", "failed", "cancelled", "paused"].includes(job.status);
}

type JobTarget = PipelineJob["targets"][number];

function TargetState({ state }: { state: JobTarget["state"] }) {
  const labels = { done: "done", running: "running", pending: "pending" };
  return <span className={`target-state target-${state}`}>{labels[state]}</span>;
}

export function Jobs() {
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const queryClient = useQueryClient();

  const { data, isLoading } = useQuery({
    queryKey: ["jobs"],
    queryFn: api.getJobs,
    refetchInterval: (query) => {
      const jobs = query.state.data;
      const hasActive = jobs?.some(
        (j) =>
          j.status === "running" ||
          j.status === "queued" ||
          j.pause_requested ||
          j.display_status === "pausing",
      );
      return hasActive ? 1000 : false;
    },
  });

  const { data: detail } = useQuery({
    queryKey: ["job", selectedId],
    queryFn: () => api.getJob(selectedId!),
    enabled: selectedId !== null,
    refetchInterval: (query) => {
      const job = query.state.data;
      if (
        job?.status === "running" ||
        job?.status === "queued" ||
        job?.pause_requested ||
        job?.display_status === "pausing"
      ) {
        return 1000;
      }
      return false;
    },
  });

  const selectedRaw = detail ?? data?.find((j) => j.id === selectedId) ?? null;
  const selected = selectedRaw ? normalizeJob(selectedRaw) : null;

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ["jobs"] });
    if (selectedId) queryClient.invalidateQueries({ queryKey: ["job", selectedId] });
  };

  const retryMutation = useMutation({
    mutationFn: api.retryJob,
    onSuccess: (res) => {
      setActionError(null);
      setSelectedId(res.jobId);
      invalidate();
    },
    onError: (e: Error) => setActionError(e.message),
  });

  const continueMutation = useMutation({
    mutationFn: api.continueJob,
    onSuccess: (res) => {
      setActionError(null);
      setSelectedId(res.jobId);
      invalidate();
    },
    onError: (e: Error) => setActionError(e.message),
  });

  const cancelMutation = useMutation({
    mutationFn: api.cancelJob,
    onSuccess: () => {
      setActionError(null);
      invalidate();
    },
    onError: (e: Error) => setActionError(e.message),
  });

  const deleteMutation = useMutation({
    mutationFn: api.deleteJob,
    onSuccess: () => {
      setActionError(null);
      setSelectedId(null);
      invalidate();
    },
    onError: (e: Error) => setActionError(e.message),
  });

  const abandonMutation = useMutation({
    mutationFn: api.abandonJob,
    onSuccess: () => {
      setActionError(null);
      invalidate();
    },
    onError: (e: Error) => setActionError(e.message),
  });

  const pauseMutation = useMutation({
    mutationFn: api.pauseJob,
    onSuccess: () => {
      setActionError(null);
      invalidate();
    },
    onError: (e: Error) => setActionError(e.message),
  });

  const resumeMutation = useMutation({
    mutationFn: api.resumeJob,
    onSuccess: (res) => {
      setActionError(null);
      setSelectedId(res.jobId);
      invalidate();
    },
    onError: (e: Error) => setActionError(e.message),
  });

  useEffect(() => {
    const active =
      data?.find((j) => j.status === "running" && !j.is_stale) ??
      data?.find((j) => j.status === "running");
    if (active) setSelectedId((prev) => prev ?? active.id);
  }, [data]);

  const handleAction = (
    e: React.MouseEvent,
    action: () => void,
    confirmMsg?: string,
  ) => {
    e.stopPropagation();
    if (confirmMsg && !window.confirm(confirmMsg)) return;
    action();
  };

  if (isLoading) return <div>Loading...</div>;

  return (
    <div>
      <div className="page-header">
        <h2>Pipeline Jobs</h2>
        <p className="page-subtitle">
          Each job runs a pipeline stage over a batch of symbols. Multiple jobs can run in parallel
          (default: 2 Cursor agents — set <code>jobs.concurrency</code> in port-harness.config.ts).
        </p>
      </div>

      {actionError && (
        <div className="warning-banner" style={{ marginBottom: "1rem" }}>
          {actionError}
          <button
            type="button"
            className="btn-link"
            onClick={() => setActionError(null)}
            style={{ marginLeft: "1rem" }}
          >
            Dismiss
          </button>
        </div>
      )}

      <table>
        <thead>
          <tr>
            <th>ID</th>
            <th>Summary</th>
            <th>Status</th>
            <th>Progress</th>
            <th>Current</th>
            <th>Elapsed</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {(data ?? []).map((j) => {
            const row = normalizeJob(j);
            return (
            <tr
              key={row.id}
              className={selectedId === row.id ? "row-selected" : undefined}
              onClick={() => setSelectedId(row.id === selectedId ? null : row.id)}
              style={{ cursor: "pointer" }}
            >
              <td>{row.id}</td>
              <td style={{ fontSize: "0.85rem", maxWidth: 280 }}>{row.summary}</td>
              <td>
                <span className={statusBadgeClass(row)}>{displayStatus(row)}</span>
              </td>
              <td>
                {row.progress}/{row.total}
                <div className="job-progress" style={{ width: 100, marginTop: 4 }}>
                  <div
                    className="job-progress-bar"
                    style={{
                      width: `${row.total ? (row.progress / row.total) * 100 : 0}%`,
                    }}
                  />
                </div>
              </td>
              <td style={{ fontSize: "0.8rem", maxWidth: 240 }}>{currentLabel(row)}</td>
              <td style={{ fontSize: "0.8rem" }}>
                {!row.is_stale && (row.status === "running" || row.status === "queued")
                  ? elapsedSince(row.created_at)
                  : ""}
              </td>
              <td onClick={(e) => e.stopPropagation()}>
                <div className="action-buttons">
                  {canPause(row) && (
                    <button
                      type="button"
                      className="btn btn-sm btn-secondary"
                      onClick={(e) => handleAction(e, () => pauseMutation.mutate(row.id))}
                    >
                      Pause
                    </button>
                  )}
                  {canResume(row) && (
                    <button
                      type="button"
                      className="btn btn-sm"
                      onClick={(e) => handleAction(e, () => resumeMutation.mutate(row.id))}
                    >
                      Resume
                    </button>
                  )}
                  {canAbandon(row) && (
                    <button
                      type="button"
                      className="btn btn-sm btn-secondary"
                      onClick={(e) =>
                        handleAction(
                          e,
                          () => abandonMutation.mutate(row.id),
                          "Mark this stuck job as failed? Use Continue to resume remaining symbols.",
                        )
                      }
                    >
                      Abandon
                    </button>
                  )}
                  {canContinue(row) && (
                    <button
                      type="button"
                      className="btn btn-sm"
                      onClick={(e) => handleAction(e, () => continueMutation.mutate(row.id))}
                    >
                      Continue
                    </button>
                  )}
                  {canRetry(row) && (
                    <button
                      type="button"
                      className="btn btn-sm btn-secondary"
                      onClick={(e) => handleAction(e, () => retryMutation.mutate(row.id))}
                    >
                      Retry
                    </button>
                  )}
                  {canCancel(row) && (
                    <button
                      type="button"
                      className="btn btn-sm btn-secondary"
                      onClick={(e) => handleAction(e, () => cancelMutation.mutate(row.id))}
                    >
                      Cancel
                    </button>
                  )}
                  {canRemove(row) && (
                    <button
                      type="button"
                      className="btn btn-sm btn-secondary"
                      onClick={(e) =>
                        handleAction(e, () => deleteMutation.mutate(row.id), "Remove this job?")
                      }
                    >
                      Remove
                    </button>
                  )}
                </div>
              </td>
            </tr>
            );
          })}
        </tbody>
      </table>

      {selected && (
        <div className="job-detail">
          <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap" }}>
            <h3>
              Job #{selected.id} — {selected.summary}
            </h3>
            <div className="action-buttons">
              {canPause(selected) && (
                <button
                  type="button"
                  className="btn btn-sm btn-secondary"
                  onClick={() => pauseMutation.mutate(selected.id)}
                >
                  Pause
                </button>
              )}
              {canResume(selected) && (
                <button
                  type="button"
                  className="btn btn-sm"
                  onClick={() => resumeMutation.mutate(selected.id)}
                >
                  Resume
                </button>
              )}
              {canAbandon(selected) && (
                <button
                  type="button"
                  className="btn btn-sm btn-secondary"
                  onClick={() => {
                    if (
                      window.confirm(
                        "Mark this stuck job as failed? Use Continue to resume remaining symbols.",
                      )
                    ) {
                      abandonMutation.mutate(selected.id);
                    }
                  }}
                >
                  Abandon
                </button>
              )}
              {canContinue(selected) && (
                <button
                  type="button"
                  className="btn btn-sm"
                  onClick={() => continueMutation.mutate(selected.id)}
                >
                  Continue
                </button>
              )}
              {canRetry(selected) && (
                <button
                  type="button"
                  className="btn btn-sm btn-secondary"
                  onClick={() => retryMutation.mutate(selected.id)}
                >
                  Retry
                </button>
              )}
              {canCancel(selected) && (
                <button
                  type="button"
                  className="btn btn-sm btn-secondary"
                  onClick={() => cancelMutation.mutate(selected.id)}
                >
                  Cancel
                </button>
              )}
              {canRemove(selected) && (
                <button
                  type="button"
                  className="btn btn-sm btn-secondary"
                  onClick={() => {
                    if (window.confirm("Remove this job?")) deleteMutation.mutate(selected.id);
                  }}
                >
                  Remove
                </button>
              )}
            </div>
          </div>

          <p style={{ color: "#94a3b8", fontSize: "0.85rem", marginBottom: "1rem" }}>
            {STAGE_DESCRIPTIONS[selected.stage] ?? selected.stage_label}
          </p>

          {(selected.pause_requested || selected.display_status === "pausing") && (
            <p style={{ color: "#fbbf24", fontSize: "0.85rem", marginBottom: "1rem" }}>
              Pause requested — the current symbol will finish, then the job will stop.
            </p>
          )}

          {selected.status === "paused" && (
            <p style={{ color: "#fbbf24", fontSize: "0.85rem", marginBottom: "1rem" }}>
              Paused at {selected.progress}/{selected.total}. Use Resume to continue from the next symbol.
            </p>
          )}

          {selected.is_stale && (
            <p style={{ color: "#fbbf24", fontSize: "0.85rem", marginBottom: "1rem" }}>
              This job is stale — no worker is processing it (likely from a crashed or duplicate server).
              Abandon it, then use Continue to resume from symbol {selected.progress + 1}.
            </p>
          )}

          {selected.error && (
            <p style={{ color: "#f87171", fontSize: "0.85rem", marginBottom: "1rem" }}>
              {selected.error}
            </p>
          )}

          {selected.targets.length > 0 && (
            <>
              <h4 style={{ marginBottom: "0.5rem", fontSize: "0.9rem" }}>Targets</h4>
              <table style={{ marginBottom: "1rem" }}>
                <thead>
                  <tr>
                    <th>Symbol</th>
                    <th>File</th>
                    <th>State</th>
                  </tr>
                </thead>
                <tbody>
                  {selected.targets.map((t) => (
                    <tr key={t.taskId}>
                      <td>{t.symbolName}</td>
                      <td style={{ fontSize: "0.85rem" }}>{t.file || "—"}</td>
                      <td>
                        <TargetState state={t.state} />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </>
          )}

          <h4 style={{ marginBottom: "0.5rem", fontSize: "0.9rem" }}>Activity log</h4>
          <pre className="job-log">{selected.log?.trim() || "No activity yet."}</pre>
        </div>
      )}
    </div>
  );
}
