import { describe, expect, it } from "vitest";
import { handleRefreshRoute } from "../../src/http/refresh-route";

describe("refresh route", () => {
  it("rotates refresh tokens", async () => {
    const token = await handleRefreshRoute("user-1");
    expect(token).toContain("rotated:user-1");
  });
});
