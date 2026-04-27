export function issueRefreshToken(userId: string): string {
  return `refresh:${userId}`;
}

export function rotateRefreshToken(userId: string): string {
  return issueRefreshToken(userId);
}

export function tokenLabel(token: string): string {
  return `token:${token}`;
}
