import { refreshSession } from "../auth/session";
import { retry } from "../lib/retry";

export async function handleRefreshRoute(userId: string): Promise<string> {
  return retry(() => Promise.resolve(refreshSession(userId)));
}
