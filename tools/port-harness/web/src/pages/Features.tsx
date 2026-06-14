import { useState } from "react";
import { Link } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../api/client";

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
        <p className="page-subtitle">Curated migration areas with suggested file, flow, and task membership.</p>
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
          {(data ?? []).map((feature) => (
            <Link key={feature.id} to={`/features/${feature.id}`} className="feature-card">
              <div className="feature-card-title">{feature.name}</div>
              <div className="muted">{feature.description || "No description"}</div>
              <div className="feature-card-stats">
                <span>{feature.total_tasks} tasks</span>
                <span>{feature.file_count} files</span>
                <span>{feature.flow_count} flows</span>
                <span>{feature.pending_suggestions} suggestions</span>
              </div>
              <div className="job-progress">
                <div
                  className="job-progress-bar"
                  style={{
                    width: `${feature.total_tasks ? (feature.done / feature.total_tasks) * 100 : 0}%`,
                  }}
                />
              </div>
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
