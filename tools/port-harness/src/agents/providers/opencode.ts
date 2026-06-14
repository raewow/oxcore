import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { writeFileSync, unlinkSync } from "node:fs";
import { join } from "node:path";
import type { z } from "zod";
import type { AgentProvider } from "../provider.js";
import { schemaToJsonSchema } from "../provider.js";

function resolveOpencodeBin(): string {
  const fromEnv = process.env.OPENCODE_BIN;
  if (fromEnv) return fromEnv;

  if (process.platform === "win32") {
    const candidates = [
      join(process.env.APPDATA ?? "", "npm", "opencode.cmd"),
      join(process.env.LOCALAPPDATA ?? "", "Microsoft", "WindowsApps", "opencode.exe"),
    ];
    for (const p of candidates) {
      if (p && existsSync(p)) return p;
    }
  }

  return "opencode";
}

function extractJson(text: string): string {
  const fenced = text.match(/```(?:json)?\s*([\s\S]*?)```/);
  if (fenced?.[1]) return fenced[1].trim();

  const brace = text.match(/\{[\s\S]*\}/);
  if (brace) return brace[0];

  return text.trim();
}

function summarizeEvent(event: { type: string; part?: Record<string, unknown> }): string | undefined {
  if (event.type === "text" && event.part?.text) {
    const t = String(event.part.text).trim().replace(/\s+/g, " ");
    return t.length > 140 ? `Agent: ${t.slice(0, 140)}…` : `Agent: ${t}`;
  }
  if (event.type === "tool_use" && event.part?.tool) {
    const state = event.part.state as { status?: string } | undefined;
    return `Tool ${event.part.tool} (${state?.status ?? "running"})`;
  }
  if (event.type === "thinking" && event.part?.text) {
    const t = String(event.part.text).trim();
    return t.length > 120 ? `Thinking: ${t.slice(0, 120)}…` : `Thinking: ${t}`;
  }
  return undefined;
}

export async function createOpencodeProvider(
  model: string | undefined,
  cwd: string,
  onActivity?: (message: string) => void,
): Promise<AgentProvider> {
  const opencodeBin = resolveOpencodeBin();

  return {
    name: "opencode",
    async completeStructured<T>(system: string, user: string, schema: z.ZodType<T>): Promise<T> {
      const jsonSchema = schemaToJsonSchema(schema);
      const prompt = [
        system,
        "",
        user,
        "",
        "Respond with valid JSON only (no markdown prose). Schema:",
        JSON.stringify(jsonSchema, null, 2),
      ].join("\n");

      // Write prompt to a temp file so we avoid command-line length limits.
      const tmpFile = join(tmpdir(), `port-harness-prompt-${Date.now()}.txt`);
      writeFileSync(tmpFile, prompt, "utf-8");

      onActivity?.("Calling opencode agent (this may take several minutes)…");

      return new Promise((resolve, reject) => {
        const args: string[] = [
          "run",
          "--format",
          "json",
          "--dangerously-skip-permissions",
          "--title",
          "port-harness",
          "Please read the attached prompt file and respond with JSON matching the schema described in it.",
          "--file",
          tmpFile,
        ];
        if (model) {
          args.push("--model", model);
        }

        const child = spawn(opencodeBin, args, {
          cwd,
          shell: true,
          env: {
            ...process.env,
            OPENCODE_DISABLE_EXTERNAL_SKILLS: "1",
          },
        });

        let buffer = "";
        let lastText = "";
        let lastLogged = "";

        child.stdout.on("data", (data: Buffer) => {
          buffer += data.toString();
          const lines = buffer.split("\n");
          buffer = lines.pop() ?? "";

          for (const line of lines) {
            if (!line.trim()) continue;
            try {
              const event = JSON.parse(line) as {
                type: string;
                part?: Record<string, unknown>;
              };

              if (event.type === "text" && event.part?.text) {
                lastText = String(event.part.text);
              }

              const summary = summarizeEvent(event);
              if (summary && summary !== lastLogged) {
                lastLogged = summary;
                onActivity?.(summary);
              }
            } catch {
              // ignore malformed JSON lines
            }
          }
        });

        let stderr = "";
        child.stderr.on("data", (data: Buffer) => {
          stderr += data.toString();
        });

        child.on("close", (code) => {
          try {
            unlinkSync(tmpFile);
          } catch {
            // ignore cleanup errors
          }

          if (code !== 0) {
            reject(new Error(`opencode exited with code ${code}: ${stderr}`));
            return;
          }

          try {
            const parsed = JSON.parse(extractJson(lastText));
            onActivity?.("Agent finished (stop)");
            resolve(schema.parse(parsed));
          } catch (err) {
            reject(
              new Error(
                `Failed to parse opencode response: ${err instanceof Error ? err.message : String(err)}`,
              ),
            );
          }
        });
      });
    },
  };
}
