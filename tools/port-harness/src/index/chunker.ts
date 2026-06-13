import type { ParsedSymbol } from "./parser.js";

export interface Chunk extends ParsedSymbol {
  parentName: string;
  chunkIndex: number;
}

export function chunkLargeMethod(
  symbol: ParsedSymbol,
  maxLines: number,
): Chunk[] {
  const lineCount = symbol.endLine - symbol.startLine + 1;
  if (lineCount <= maxLines) {
    return [];
  }

  const chunks: Chunk[] = [];
  let chunkIndex = 0;
  let start = symbol.startLine;

  while (start <= symbol.endLine) {
    const end = Math.min(start + maxLines - 1, symbol.endLine);
    const shortName = symbol.name.split("::").pop() ?? symbol.name;
    chunks.push({
      file: symbol.file,
      name: `${symbol.name}:chunk_${chunkIndex}`,
      kind: "method",
      startLine: start,
      endLine: end,
      parentName: symbol.name,
      chunkIndex,
    });
    start = end + 1;
    chunkIndex++;
  }

  return chunks;
}

export function shouldChunk(symbol: ParsedSymbol, maxLines: number): boolean {
  return symbol.endLine - symbol.startLine + 1 > maxLines;
}
