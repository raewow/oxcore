import { createRequire } from "node:module";
import { readFileSync, existsSync } from "node:fs";
import { basename } from "node:path";
import type Parser from "tree-sitter";

const require = createRequire(import.meta.url);

export interface ParsedSymbol {
  file: string;
  name: string;
  kind: "function" | "class" | "struct" | "enum" | "method";
  startLine: number;
  endLine: number;
}

export interface ParsedCall {
  calleeName: string;
  line: number;
}

let parser: Parser | null = null;

function getParser(): Parser {
  if (!parser) {
    const ParserCtor = require("tree-sitter") as { new (): Parser };
    const Cpp = require("tree-sitter-cpp");
    const instance = new ParserCtor();
    instance.setLanguage(Cpp);
    parser = instance;
  }
  return parser;
}

export function parseFile(filePath: string): Parser.Tree {
  const source = readFileSync(filePath, "utf-8");
  return getParser().parse(source);
}

export function getSourceLines(filePath: string): string[] {
  return readFileSync(filePath, "utf-8").split("\n");
}

export function extractSymbolsFromHeader(filePath: string): ParsedSymbol[] {
  const tree = parseFile(filePath);
  const file = basename(filePath);
  const symbols: ParsedSymbol[] = [];
  const source = readFileSync(filePath, "utf-8");

  function walk(node: Parser.SyntaxNode, className?: string) {
    if (node.type === "class_specifier") {
      const nameNode = node.childForFieldName("name");
      const classNameStr = nameNode?.text ?? "unknown";
      symbols.push({
        file,
        name: classNameStr,
        kind: "class",
        startLine: node.startPosition.row + 1,
        endLine: node.endPosition.row + 1,
      });
      for (const child of node.children) {
        walk(child, classNameStr);
      }
      return;
    }

    if (node.type === "struct_specifier") {
      const nameNode = node.childForFieldName("name");
      symbols.push({
        file,
        name: nameNode?.text ?? "unknown",
        kind: "struct",
        startLine: node.startPosition.row + 1,
        endLine: node.endPosition.row + 1,
      });
      return;
    }

    if (node.type === "enum_specifier") {
      const nameNode = node.childForFieldName("name");
      symbols.push({
        file,
        name: nameNode?.text ?? "unknown",
        kind: "enum",
        startLine: node.startPosition.row + 1,
        endLine: node.endPosition.row + 1,
      });
      return;
    }

    if (node.type === "function_definition") {
      const decl = node.childForFieldName("declarator");
      const name = extractFunctionName(decl);
      if (name) {
        symbols.push({
          file,
          name: className ? `${className}::${name}` : name,
          kind: className ? "method" : "function",
          startLine: node.startPosition.row + 1,
          endLine: node.endPosition.row + 1,
        });
      }
      return;
    }

    if (node.type === "declaration" && className) {
      const fnDecl = findFunctionDeclarator(node);
      if (fnDecl) {
        const name = extractFunctionName(fnDecl);
        if (name && !name.startsWith("Effect")) {
          symbols.push({
            file,
            name: `${className}::${name}`,
            kind: "method",
            startLine: node.startPosition.row + 1,
            endLine: node.endPosition.row + 1,
          });
        }
      }
    }

    for (const child of node.children) {
      walk(child, className);
    }
  }

  walk(tree.rootNode);
  return symbols;
}

function findFunctionDeclarator(node: Parser.SyntaxNode): Parser.SyntaxNode | null {
  if (node.type === "function_declarator") return node;
  for (const child of node.children) {
    const found = findFunctionDeclarator(child);
    if (found) return found;
  }
  return null;
}

function extractFunctionName(declarator: Parser.SyntaxNode | null): string | null {
  if (!declarator) return null;

  if (declarator.type === "identifier") {
    return declarator.text;
  }

  if (declarator.type === "qualified_identifier") {
    const name = declarator.childForFieldName("name");
    return name?.text ?? null;
  }

  if (declarator.type === "field_identifier") {
    return declarator.text;
  }

  if (declarator.type === "destructor_name") {
    return declarator.text;
  }

  if (declarator.type === "operator_name") {
    return `operator${declarator.text.replace("operator", "")}`;
  }

  if (declarator.type === "pointer_declarator" || declarator.type === "reference_declarator") {
    const inner = declarator.childForFieldName("declarator");
    return extractFunctionName(inner);
  }

  if (declarator.type === "function_declarator") {
    const inner = declarator.childForFieldName("declarator");
    return extractFunctionName(inner);
  }

  for (const child of declarator.children) {
    const name = extractFunctionName(child);
    if (name) return name;
  }

  return null;
}

export function findMethodDefinitionInCpp(
  cppPath: string,
  methodName: string,
  className: string,
): { startLine: number; endLine: number } | null {
  const source = readFileSync(cppPath, "utf-8");
  const lines = source.split("\n");
  const shortName = methodName.includes("::") ? methodName.split("::").pop()! : methodName;
  const pattern = new RegExp(`\\b${className}::${escapeRegex(shortName)}\\s*\\(`);

  let startLine = -1;
  let braceDepth = 0;
  let foundOpen = false;

  for (let i = 0; i < lines.length; i++) {
    if (startLine === -1) {
      if (pattern.test(lines[i]!)) {
        startLine = i + 1;
      }
      continue;
    }

    for (const ch of lines[i]!) {
      if (ch === "{") {
        braceDepth++;
        foundOpen = true;
      } else if (ch === "}") {
        braceDepth--;
      }
    }

    if (foundOpen && braceDepth === 0) {
      return { startLine, endLine: i + 1 };
    }
  }

  if (startLine !== -1) {
    return { startLine, endLine: Math.min(startLine + 50, lines.length) };
  }

  return null;
}

function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function extractCallsFromRange(
  filePath: string,
  startLine: number,
  endLine: number,
): ParsedCall[] {
  const lines = getSourceLines(filePath);
  const calls: ParsedCall[] = [];
  const callPattern = /(\w+(?:->|\.)?\w*)\s*\(/g;

  for (let i = startLine - 1; i < Math.min(endLine, lines.length); i++) {
    const line = lines[i]!;
    let match;
    while ((match = callPattern.exec(line)) !== null) {
      const callee = match[1]!;
      if (!["if", "while", "for", "switch", "return", "sizeof", "catch"].includes(callee)) {
        calls.push({ calleeName: callee, line: i + 1 });
      }
    }
  }

  return calls;
}

export function getSourceSnippet(
  filePath: string,
  startLine: number,
  endLine: number,
): string {
  if (!existsSync(filePath)) {
    throw new Error(`Source file not found: ${filePath}`);
  }
  const lines = getSourceLines(filePath);
  return lines.slice(startLine - 1, endLine).join("\n");
}

export function extractIncludes(filePath: string): string[] {
  const source = readFileSync(filePath, "utf-8");
  const includes: string[] = [];
  const pattern = /#include\s*[<"]([^>"]+)[>"]/g;
  let match;
  while ((match = pattern.exec(source)) !== null) {
    includes.push(match[1]!);
  }
  return includes;
}

export { extractMethodsFromCpp } from "./cppMethods.js";

export function extractClassMethodsFromHeader(
  headerPath: string,
  className: string,
  excludePatterns: string[],
): ParsedSymbol[] {
  const all = extractSymbolsFromHeader(headerPath);
  return all.filter((s) => {
    if (s.kind !== "method") return false;
    if (!s.name.startsWith(`${className}::`)) return false;
    const shortName = s.name.split("::").pop()!;
    for (const pattern of excludePatterns) {
      if (pattern.endsWith("*") && shortName.startsWith(pattern.slice(0, -1))) return false;
      if (shortName === pattern) return false;
    }
    return true;
  });
}
