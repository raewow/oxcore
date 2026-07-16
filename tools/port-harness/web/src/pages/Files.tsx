import { useMemo, useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { api, type FileListEntry } from "../api/client";

type SortKey = "path" | "size" | "symbols" | "progress";
type SortDir = "asc" | "desc";

function SortIndicator({ active, dir }: { active: boolean; dir: SortDir }) {
  if (!active) return <span className="th-sort" aria-hidden>⇅</span>;
  return (
    <span className="th-sort th-sort-active" aria-hidden>{dir === "desc" ? "▼" : "▲"}</span>
  );
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

function progressPct(file: FileListEntry): number {
  if (!file.symbol_count) return 0;
  return Math.round((file.documented / file.symbol_count) * 100);
}

const STATUS_ORDER = [
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
] as const;

const STATUS_META: Record<string, { color: string; label: string; portedPlus: boolean }> = {
  discovered: { color: "#475569", label: "Discovered", portedPlus: false },
  documented: { color: "#60a5fa", label: "Documented", portedPlus: false },
  fixture_defined: { color: "#fbbf24", label: "Fixture", portedPlus: false },
  rust_planned: { color: "#f97316", label: "Planned", portedPlus: false },
  rust_ported: { color: "#a5b4fc", label: "Ported", portedPlus: true },
  rust_compiled: { color: "#818cf8", label: "Compiled", portedPlus: true },
  verified: { color: "#34d399", label: "Verified", portedPlus: true },
  reviewed: { color: "#10b981", label: "Reviewed", portedPlus: true },
  done: { color: "#059669", label: "Done", portedPlus: true },
  blocked: { color: "#ef4444", label: "Blocked", portedPlus: false },
};

const PORTED_PLUS_STATUSES = STATUS_ORDER.filter((s) => STATUS_META[s].portedPlus);

function portedPlusCount(file: FileListEntry): number {
  return PORTED_PLUS_STATUSES.reduce(
    (sum, s) => sum + (file.by_status?.[s] ?? 0),
    0,
  );
}

function portedPlusPct(file: FileListEntry): number {
  if (!file.symbol_count) return 0;
  return Math.round((portedPlusCount(file) / file.symbol_count) * 100);
}

function segPct(file: FileListEntry, status: string): number {
  if (!file.symbol_count) return 0;
  return ((file.by_status?.[status] ?? 0) / file.symbol_count) * 100;
}

export function Files() {
  const [q, setQ] = useState("");
  const [kind, setKind] = useState<"" | "cpp" | "h">("cpp");
  const [sortKey, setSortKey] = useState<SortKey>("path");
  const [sortDir, setSortDir] = useState<SortDir>("asc");
  const [busyPath, setBusyPath] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const queryClient = useQueryClient();

  const { data, isLoading, refetch } = useQuery({
    queryKey: ["files", q, kind],
    queryFn: () => api.getFiles({ q: q || undefined, kind: kind || undefined }),
  });

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ["files"] });
    queryClient.invalidateQueries({ queryKey: ["tasks"] });
    queryClient.invalidateQueries({ queryKey: ["stats"] });
    queryClient.invalidateQueries({ queryKey: ["jobs"] });
  };

  const indexMutation = useMutation({
    mutationFn: (path: string) => api.indexFile(path),
    onMutate: (path) => setBusyPath(path),
    onSuccess: (res) => {
      setMessage(`Index job #${res.jobId} queued for ${res.path}. See Jobs tab.`);
      invalidate();
    },
    onError: (e: Error) => setMessage(e.message),
    onSettled: () => setBusyPath(null),
  });

  const documentMutation = useMutation({
    mutationFn: (path: string) => api.documentFile(path),
    onMutate: (path) => setBusyPath(path),
    onSuccess: (res) => {
      setMessage(
        `Queued ${res.totalTasks} symbols in ${res.batches} job(s). See Jobs tab.`,
      );
      invalidate();
    },
    onError: (e: Error) => setMessage(e.message),
    onSettled: () => setBusyPath(null),
  });

  const flowsMutation = useMutation({
    mutationFn: (path: string) => api.assembleFlowsForFile(path),
    onMutate: (path) => setBusyPath(path),
    onSuccess: () => {
      setMessage("Assemble flows job queued. See Jobs tab.");
      invalidate();
    },
    onError: (e: Error) => setMessage(e.message),
    onSettled: () => setBusyPath(null),
  });

  const pipelineMutation = useMutation({
    mutationFn: (path: string) => api.runFilePipeline(path),
    onMutate: (path) => setBusyPath(path),
    onSuccess: (res) => {
      setMessage(`Pipeline job #${res.jobId} queued (index + extract + flows). See Jobs tab.`);
      invalidate();
    },
    onError: (e: Error) => setMessage(e.message),
    onSettled: () => setBusyPath(null),
  });

  const toggleSort = (key: SortKey) => {
    if (sortKey === key) {
      setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    } else {
      setSortKey(key);
      setSortDir(key === "path" ? "asc" : "desc");
    }
  };

  const sortedFiles = useMemo(() => {
    const files = data?.files ?? [];
    const factor = sortDir === "asc" ? 1 : -1;
    const compare = (a: FileListEntry, b: FileListEntry): number => {
      switch (sortKey) {
        case "path":
          return a.path.localeCompare(b.path);
        case "size":
          return a.size_bytes - b.size_bytes;
        case "symbols":
          return (a.symbol_count ?? 0) - (b.symbol_count ?? 0);
        case "progress":
          return portedPlusPct(a) - portedPlusPct(b);
      }
    };
    return [...files].sort((a, b) => compare(a, b) * factor);
  }, [data?.files, sortKey, sortDir]);

  const confirmAndRun = (
    label: string,
    path: string,
    fn: (path: string) => void,
    estimate?: string,
  ) => {
    if (window.confirm(`${label} for ${path}?${estimate ? `\n\n${estimate}` : ""}`)) {
      fn(path);
    }
  };

  return (
    <div>
      <div className="page-header">
        <h2>Files</h2>
        <p style={{ color: "#94a3b8" }}>
          Browse reference C++ under <code>reference/core/</code>. Index parses symbols;
          Document runs Cursor extract on all discovered symbols; Assemble Flows groups them.
        </p>
      </div>

      {message && (
        <div className="info-banner">
          {message}
          <button type="button" className="btn-link" onClick={() => setMessage(null)}>
            dismiss
          </button>
        </div>
      )}

      <div className="filters">
        <input
          placeholder="Search files (e.g. Unit, game/Entities)..."
          value={q}
          onChange={(e) => setQ(e.target.value)}
          style={{ minWidth: 280 }}
        />
        <select value={kind} onChange={(e) => setKind(e.target.value as "" | "cpp" | "h")}>
          <option value="">All types</option>
          <option value="cpp">.cpp only</option>
          <option value="h">.h only</option>
        </select>
        <button type="button" className="btn btn-secondary" onClick={() => refetch()}>
          Refresh
        </button>
        <span style={{ color: "#94a3b8", fontSize: "0.875rem" }}>
          {data?.total ?? 0} files · {data?.indexed_count ?? 0} indexed
        </span>
      </div>

      {isLoading ? (
        <div>Loading files...</div>
      ) : (
        <table>
          <thead>
            <tr>
              {(["path", "size", "symbols", "progress"] as SortKey[]).map((key) => (
                <th
                  key={key}
                  className="th-sortable"
                  onClick={() => toggleSort(key)}
                  style={{ cursor: "pointer", userSelect: "none" }}
                >
                  <span style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}>
                    {key === "path" ? "Path" : key === "size" ? "Size" : key === "symbols" ? "Symbols" : "Progress"}
                    <SortIndicator active={sortKey === key} dir={sortDir} />
                  </span>
                </th>
              ))}
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {sortedFiles.map((file) => (
              <tr key={file.path}>
                <td>
                  <Link to={`/files/detail?path=${encodeURIComponent(file.path)}`}>
                    <code className="file-path">{file.path}</code>
                  </Link>
                  {!file.indexed && file.kind === "cpp" && (
                    <span className="badge badge-discovered" style={{ marginLeft: 8 }}>
                      not indexed
                    </span>
                  )}
                </td>
                <td>{formatBytes(file.size_bytes)}</td>
                <td>
                  {file.indexed ? (
                    <>
                      {file.documented}/{file.symbol_count}
                      {file.discovered > 0 && (
                        <span style={{ color: "#94a3b8" }}> ({file.discovered} todo)</span>
                      )}
                    </>
                  ) : (
                    "—"
                  )}
                </td>
                <td style={{ minWidth: 120 }}>
                  {file.indexed ? (
                    <>
                      {progressPct(file)}%
                      <div className="job-progress">
                        <div
                          className="job-progress-bar"
                          style={{ width: `${progressPct(file)}%` }}
                        />
                      </div>
                    </>
                  ) : (
                    "—"
                  )}
                </td>
                <td>
                  <div className="action-buttons">
                    {file.kind === "cpp" && (
                      <>
                        <button
                          type="button"
                          className="btn btn-secondary btn-sm"
                          disabled={busyPath === file.path}
                          onClick={() => indexMutation.mutate(file.path)}
                        >
                          Index
                        </button>
                        <button
                          type="button"
                          className="btn btn-secondary btn-sm"
                          disabled={!file.indexed || busyPath === file.path}
                          onClick={() =>
                            confirmAndRun(
                              "Document all discovered symbols",
                              file.path,
                              (p) => documentMutation.mutate(p),
                              `Up to ${file.discovered} Cursor API calls (batched).`,
                            )
                          }
                        >
                          Document
                        </button>
                        <button
                          type="button"
                          className="btn btn-secondary btn-sm"
                          disabled={!file.documented || busyPath === file.path}
                          onClick={() =>
                            confirmAndRun("Assemble flows", file.path, (p) =>
                              flowsMutation.mutate(p),
                            )
                          }
                        >
                          Flows
                        </button>
                        <button
                          type="button"
                          className="btn btn-sm"
                          disabled={busyPath === file.path}
                          onClick={() =>
                            confirmAndRun(
                              "Run full pipeline (index + document + flows)",
                              file.path,
                              (p) => pipelineMutation.mutate(p),
                              file.discovered
                                ? `Will queue ~${file.discovered} extract jobs after index.`
                                : "Will index then queue extract jobs for all symbols.",
                            )
                          }
                        >
                          All
                        </button>
                      </>
                    )}
                    {file.indexed && (
                      <Link
                        to={`/tasks?file=${encodeURIComponent(file.name)}`}
                        className="btn btn-secondary btn-sm"
                      >
                        Tasks
                      </Link>
                    )}
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
