interface Props {
  selectedCount: number;
  onSetStatus: (status: string) => void;
  onQueueJob: (stage: string) => void;
  onClear: () => void;
}

export function BulkActionBar({ selectedCount, onSetStatus, onQueueJob, onClear }: Props) {
  if (selectedCount === 0) return null;

  return (
    <div className="bulk-bar">
      <span>{selectedCount} selected</span>
      <select
        onChange={(e) => {
          if (e.target.value) onSetStatus(e.target.value);
          e.target.value = "";
        }}
        defaultValue=""
      >
        <option value="">Set status...</option>
        <option value="reviewed">reviewed</option>
        <option value="blocked">blocked</option>
        <option value="done">done</option>
      </select>
      <button className="btn btn-secondary" onClick={() => onQueueJob("extract")}>
        Queue Extract
      </button>
      <button className="btn btn-secondary" onClick={() => onQueueJob("plan-rust")}>
        Queue Plan
      </button>
      <button className="btn btn-secondary" onClick={() => onQueueJob("audit-rust")}>
        Queue Audit
      </button>
      <button className="btn btn-secondary" onClick={() => onQueueJob("verify")}>
        Queue Verify
      </button>
      <button className="btn btn-secondary" onClick={onClear}>
        Clear
      </button>
    </div>
  );
}
