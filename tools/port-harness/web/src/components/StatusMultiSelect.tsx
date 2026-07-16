import { useEffect, useRef, useState } from "react";
import { StatusBadge } from "./StatusBadge";

const ALL_STATUSES = [
  "discovered",
  "documented",
  "fixture_defined",
  "rust_planned",
  "rust_ported",
  "verified",
  "done",
  "blocked",
];

interface Props {
  selected: string[];
  onChange: (next: string[]) => void;
  statuses?: string[];
}

export function StatusMultiSelect({ selected, onChange, statuses = ALL_STATUSES }: Props) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function onDocClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener("mousedown", onDocClick);
    return () => document.removeEventListener("mousedown", onDocClick);
  }, []);

  const toggle = (status: string) => {
    if (selected.includes(status)) {
      onChange(selected.filter((s) => s !== status));
    } else {
      onChange([...selected, status]);
    }
  };

  const label =
    selected.length === 0
      ? "All statuses"
      : selected.length === 1
        ? selected[0].replace(/_/g, " ")
        : `${selected.length} statuses`;

  return (
    <div ref={ref} className="multiselect">
      <button
        type="button"
        className="multiselect-trigger"
        onClick={() => setOpen((v) => !v)}
      >
        <span>{label}</span>
        <span style={{ marginLeft: "0.4rem", fontSize: "0.7rem", color: "#94a3b8" }}>
          {open ? "▲" : "▼"}
        </span>
      </button>
      {open && (
        <div className="multiselect-panel">
          <div className="multiselect-actions">
            <button
              type="button"
              className="btn btn-secondary btn-sm"
              onClick={() => onChange([])}
              disabled={selected.length === 0}
            >
              Clear
            </button>
            <button
              type="button"
              className="btn btn-secondary btn-sm"
              onClick={() => onChange(statuses.filter((s) => !selected.includes(s)))}
            >
              Invert
            </button>
          </div>
          {statuses.map((s) => (
            <label key={s} className="multiselect-option">
              <input
                type="checkbox"
                checked={selected.includes(s)}
                onChange={() => toggle(s)}
              />
              <StatusBadge status={s} />
            </label>
          ))}
        </div>
      )}
    </div>
  );
}