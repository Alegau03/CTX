import { describe, expect, it } from "vitest";
import { refreshSession } from "../../src/auth/session";

describe("session", () => {
  it("labels the refreshed session token", () => {
    expect(refreshSession("user-2")).toContain("token:refresh:user-2");
  });
});
