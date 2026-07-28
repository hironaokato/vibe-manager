import { describe, expect, it } from "vitest";
import { canStart, canStop, formatRelative, shortPath } from "./format";

describe("formatRelative", () => {
  const now = 1_700_000_000_000;

  it("formats recent times in English followed by Japanese", () => {
    expect(formatRelative(now - 5_000, now)).toBe("Just now / たった今");
    expect(formatRelative(now - 120_000, now)).toBe("2m ago / 2分前");
    expect(formatRelative(now - 7_200_000, now)).toBe("2h ago / 2時間前");
  });
});

describe("shortPath", () => {
  it("keeps short paths unchanged", () => {
    expect(shortPath("C:\\code\\app")).toBe("C:\\code\\app");
  });

  it("shortens the middle of a long path", () => {
    const result = shortPath(
      "C:\\Users\\person\\Documents\\GitHub\\a-very-long-project\\frontend",
      35,
    );
    expect(result).toContain("…");
    expect(result).toContain("frontend");
  });
});

describe("status actions", () => {
  it("only permits sensible start and stop actions", () => {
    expect(canStart("restorePending")).toBe(true);
    expect(canStart("running")).toBe(false);
    expect(canStop("running")).toBe(true);
    expect(canStop("crashed")).toBe(false);
  });
});
