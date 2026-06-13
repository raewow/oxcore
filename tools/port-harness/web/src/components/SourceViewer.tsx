interface Props {
  snippet: string;
  highlightStart?: number;
  highlightEnd?: number;
  baseLine?: number;
}

export function SourceViewer({
  snippet,
  highlightStart,
  highlightEnd,
  baseLine = 1,
}: Props) {
  const lines = snippet.split("\n");

  return (
    <div className="source-viewer">
      {lines.map((line, i) => {
        const lineNum = baseLine + i;
        const highlighted =
          highlightStart !== undefined &&
          highlightEnd !== undefined &&
          lineNum >= highlightStart &&
          lineNum <= highlightEnd;

        return (
          <div key={i} className={highlighted ? "highlight-line" : undefined}>
            <span style={{ color: "#64748b", marginRight: "1rem" }}>
              {String(lineNum).padStart(5)}
            </span>
            {line}
          </div>
        );
      })}
    </div>
  );
}
