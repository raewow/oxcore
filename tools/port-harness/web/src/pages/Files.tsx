import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { api, type FileListEntry } from "../api/client";

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

function progressPct(file: FileListEntry): number {
  if (!file.symbol_count) return 0;
  return Math.round((file.documented / file.symbol_count) * 100);
}

export function Files() {
  const [q, setQ] = useState("");
  const [kind, setKind] = useState<"" | "cpp" | "h">("cpp");
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
              <th>Path</th>
              <th>Size</th>
              <th>Symbols</th>
              <th>Progress</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {(data?.files ?? []).map((file) => (
              <tr key={file.path}>
                <td>
                  <code className="file-path">{file.path}</code>
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
