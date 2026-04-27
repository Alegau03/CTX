export function appendAuditEvent(kind: string, userId: string): string {
  return `${kind}:${userId}`;
}
