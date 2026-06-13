/** Strip harness trace comments from generated port output. Keeps TODO comments. */
export function cleanPortRustCode(code: string): string {
  const lines = code.split("\n");
  const cleaned: string[] = [];

  for (const line of lines) {
    const trimmed = line.trimStart();
    if (/^\/\/\s*PORT:/i.test(trimmed)) continue;
    if (/^\/\/\/\s*C\+\+/.test(trimmed)) continue;
    cleaned.push(line);
  }

  return cleaned.join("\n").replace(/\n{4,}/g, "\n\n\n");
}
