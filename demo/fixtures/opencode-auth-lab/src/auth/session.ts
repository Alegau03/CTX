import { appendAuditEvent } from "./audit";
import { rotateRefreshToken, tokenLabel } from "./tokens";

export function refreshSession(userId: string): string {
  const token = rotateRefreshToken(userId);
  appendAuditEvent("refresh", userId);
  return tokenLabel(token);
}
