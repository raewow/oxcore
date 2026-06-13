import type { BehaviourClaim } from "../models/index.js";
import { BehaviourClaimSchema } from "../models/index.js";

export interface CitationValidationResult {
  valid: boolean;
  errors: string[];
  claims: BehaviourClaim[];
}

export function validateClaims(
  claims: Omit<BehaviourClaim, "id">[],
  sourceFile: string,
  sourceStartLine: number,
  sourceEndLine: number,
): CitationValidationResult {
  const errors: string[] = [];
  const validated: BehaviourClaim[] = [];

  for (let i = 0; i < claims.length; i++) {
    const claim = claims[i]!;
    const result = BehaviourClaimSchema.safeParse(claim);

    if (!result.success) {
      errors.push(`Claim ${i}: ${result.error.message}`);
      continue;
    }

    const c = result.data;

    if (!c.file || c.file.trim() === "") {
      errors.push(`Claim ${i}: missing file citation`);
      continue;
    }

    if (c.start_line <= 0 || c.end_line <= 0) {
      errors.push(`Claim ${i}: invalid line range`);
      continue;
    }

    if (c.start_line > c.end_line) {
      errors.push(`Claim ${i}: start_line > end_line`);
      continue;
    }

    if (c.start_line < sourceStartLine || c.end_line > sourceEndLine) {
      errors.push(
        `Claim ${i}: line range ${c.start_line}-${c.end_line} outside symbol range ${sourceStartLine}-${sourceEndLine}`,
      );
    }

    validated.push(c as BehaviourClaim);
  }

  return {
    valid: errors.length === 0,
    errors,
    claims: validated,
  };
}

export function formatCitation(file: string, startLine: number, endLine: number): string {
  if (startLine === endLine) {
    return `${file}:${startLine}`;
  }
  return `${file}:${startLine}-${endLine}`;
}
