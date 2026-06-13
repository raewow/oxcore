import type { z } from "zod";
import type { SDKMessage } from "@cursor/sdk";
import type { AgentProvider } from "../provider.js";
import { schemaToJsonSchema } from "../provider.js";

function extractJson(text: string): string {
  const fenced = text.match(/```(?:json)?\s*([\s\S]*?)```/);
  if (fenced?.[1]) return fenced[1].trim();

  const brace = text.match(/\{[\s\S]*\}/);
  if (brace) return brace[0];

  return text.trim();
}

function summarizeStreamEvent(event: SDKMessage): string | undefined {
  switch (event.type) {
    case "status":
      return event.message ? `Status: ${event.status} — ${event.message}` : `Status: ${event.status}`;
    case "tool_call":
      return `Tool ${event.name} (${event.status})`;
    case "thinking":
      return event.text ? `Thinking: ${event.text.slice(0, 120)}` : "Thinking…";
    case "task":
      return event.text ? `Task: ${event.text.slice(0, 120)}` : event.status ? `Task: ${event.status}` : undefined;
    case "assistant": {
      for (const block of event.message.content) {
        if (block.type === "text" && block.text.trim()) {
          const t = block.text.trim().replace(/\s+/g, " ");
          return t.length > 140 ? `Agent: ${t.slice(0, 140)}…` : `Agent: ${t}`;
        }
      }
      return undefined;
    }
    default:
      return undefined;
  }
}

export async function createCursorProvider(
  model: string,
  apiKey: string,
  cwd: string,
  onActivity?: (message: string) => void,
): Promise<AgentProvider> {
  const { Agent } = await import("@cursor/sdk");

  return {
    name: "cursor",
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

      onActivity?.("Calling Cursor agent (this may take several minutes)…");

      await using agent = await Agent.create({
        apiKey,
        model: { id: model },
        local: { cwd },
      });

      const run = await agent.send(prompt);
      let lastLogged = "";

      for await (const event of run.stream()) {
        const summary = summarizeStreamEvent(event);
        if (summary && summary !== lastLogged) {
          lastLogged = summary;
          onActivity?.(summary);
        }
      }

      const result = await run.wait();

      if (result.status === "error") {
        throw new Error(`Cursor agent run failed: ${result.id ?? "unknown run"}`);
      }

      onActivity?.(`Agent finished (${result.status})`);

      const raw =
        typeof result.result === "string" ? result.result : JSON.stringify(result.result ?? {});
      const parsed = JSON.parse(extractJson(raw));
      return schema.parse(parsed);
    },
  };
}
