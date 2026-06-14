import { useState } from "react";
import { Link } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../api/client";

const RISK_COLORS: Record<string, string> = {
  critical: "#ef4444",
  high: "#f97316",
  medium: "#eab308",
  low: "#22c55e",
};

export function Features() {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const queryClient = useQueryClient();
  const { data, isLoading } = useQuery({ queryKey: ["features"], queryFn: api.getFeatures });
  const createMutation = useMutation({
    mutationFn: () => api.createFeature(name, description),
    onSuccess: () => {
      setName("");
      setDescription("");
      queryClient.invalidateQueries({ queryKey: ["features"] });
    },
  });

  return (
    <div>
      <div className="page-header">
        <h2>Features</h2>
        <p className="page-subtitle">Migration areas grouped by game feature. Click a feature to manage flows, run discovery, and track porting progress.</p>
      </div>

      <div className="feature-create">
        <input placeholder="Feature name" value={name} onChange={(e) => setName(e.target.value)} />
        <input
          placeholder="Description or keywords"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />
        <button className="btn btn-sm" disabled={!name.trim()} onClick={() => createMutation.mutate()}>
          Create
        </button>
      </div>

      {isLoading ? (
        <div>Loading features...</div>
      ) : (
        <div className="feature-grid">
          {(data ?? []).map((feature) => {
            const pct = feature.total_tasks ? Math.round((feature.done / feature.total_tasks) * 100) : 0;
            return (
              <Link key={feature.id} to={`/features/${feature.id}`} className="feature-card">
                <div className="feature-card-title">{feature.name}</div>
                {feature.description && <div className="muted" style={{ fontSize: "0.8rem", marginBottom: "0.5rem" }}>{feature.description}</div>}

                <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginBottom: "0.4rem" }}>
                  <span style={{ fontSize: "1.1rem", fontWeight: 600 }}>{feature.flow_count}</span>
                  <span className="muted" style={{ fontSize: "0.85rem" }}>flows</span>
                  {feature.file_count > 0 && (
                    <span className="muted" style={{ fontSize: "0.8rem", marginLeft: "0.5rem" }}>{feature.file_count} files</span>
                  )}
                  {feature.pending_suggestions > 0 && (
                    <span className="badge badge-discovered" style={{ marginLeft: "auto" }}>
                      {feature.pending_suggestions} suggestions
                    </span>
                  )}
                </div>

                <div className="job-progress" style={{ marginBottom: "0.4rem" }}>
                  <div
                    className="job-progress-bar"
                    style={{ width: `${pct}%` }}
                  />
                </div>

                <div className="muted" style={{ fontSize: "0.75rem" }}>
                  {feature.total_tasks} tasks &middot; {feature.done} done
                  {feature.blocked > 0 && <span style={{ color: "#f87171" }}> &middot; {feature.blocked} blocked</span>}
                </div>
              </Link>
            );
          })}
        </div>
      )}
    </div>
  );
}
