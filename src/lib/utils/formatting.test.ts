import { describe, it, expect } from "vitest";
import {
  getUsageColor,
  formatResetTime,
  formatLastUpdate,
  formatSecondsAgo,
  formatCountdown,
} from "./formatting";

describe("getUsageColor", () => {
  it("returns green for usage below 50%", () => {
    expect(getUsageColor(0)).toBe("green");
    expect(getUsageColor(25)).toBe("green");
    expect(getUsageColor(49)).toBe("green");
  });

  it("returns yellow for usage 50-79%", () => {
    expect(getUsageColor(50)).toBe("yellow");
    expect(getUsageColor(65)).toBe("yellow");
    expect(getUsageColor(79)).toBe("yellow");
  });

  it("returns orange for usage 80-89%", () => {
    expect(getUsageColor(80)).toBe("orange");
    expect(getUsageColor(85)).toBe("orange");
    expect(getUsageColor(89)).toBe("orange");
  });

  it("returns red for usage 90% and above", () => {
    expect(getUsageColor(90)).toBe("red");
    expect(getUsageColor(95)).toBe("red");
    expect(getUsageColor(100)).toBe("red");
  });
});

describe("formatResetTime", () => {
  it("formats minutes only when less than 1 hour", () => {
    const now = new Date();
    const future = new Date(now.getTime() + 30 * 60 * 1000 + 500); // 30 minutes + buffer
    const result = formatResetTime(future.toISOString());
    expect(result).toMatch(/^(29|30)m$/);
  });

  it("formats hours and minutes when less than 24 hours", () => {
    const now = new Date();
    const future = new Date(now.getTime() + 2 * 60 * 60 * 1000 + 15 * 60 * 1000 + 500); // 2h 15m + buffer
    const result = formatResetTime(future.toISOString());
    expect(result).toMatch(/^2h (14|15)m$/);
  });

  it("formats days and hours when more than 24 hours", () => {
    const now = new Date();
    const future = new Date(now.getTime() + 26 * 60 * 60 * 1000 + 500); // 26 hours + buffer
    const result = formatResetTime(future.toISOString());
    expect(result).toMatch(/^1d (1|2)h$/);
  });

  it("returns empty string for invalid date", () => {
    expect(formatResetTime("invalid-date")).toBe("");
  });
});

describe("formatLastUpdate", () => {
  it("returns 'Never' for null date", () => {
    expect(formatLastUpdate(null)).toBe("Never");
  });

  it("returns 'Just now' for recent updates", () => {
    const now = new Date();
    expect(formatLastUpdate(now)).toBe("Just now");
  });

  it("returns '1 min ago' for one minute old", () => {
    const oneMinAgo = new Date(Date.now() - 60 * 1000);
    expect(formatLastUpdate(oneMinAgo)).toBe("1 min ago");
  });

  it("returns 'X min ago' for older updates", () => {
    const fiveMinAgo = new Date(Date.now() - 5 * 60 * 1000);
    expect(formatLastUpdate(fiveMinAgo)).toBe("5 min ago");
  });
});

describe("formatSecondsAgo", () => {
  it("returns 'Just now' for less than 60 seconds", () => {
    expect(formatSecondsAgo(0)).toBe("Just now");
    expect(formatSecondsAgo(30)).toBe("Just now");
    expect(formatSecondsAgo(59)).toBe("Just now");
  });

  it("returns '1 min ago' for 60-119 seconds", () => {
    expect(formatSecondsAgo(60)).toBe("1 min ago");
    expect(formatSecondsAgo(90)).toBe("1 min ago");
    expect(formatSecondsAgo(119)).toBe("1 min ago");
  });

  it("returns 'X min ago' for longer periods", () => {
    expect(formatSecondsAgo(120)).toBe("2 min ago");
    expect(formatSecondsAgo(300)).toBe("5 min ago");
    expect(formatSecondsAgo(600)).toBe("10 min ago");
  });
});

describe("formatCountdown", () => {
  it("returns 'now' for zero or negative seconds", () => {
    expect(formatCountdown(0)).toBe("now");
    expect(formatCountdown(-5)).toBe("now");
  });

  it("returns seconds only when less than 60", () => {
    expect(formatCountdown(1)).toBe("1s");
    expect(formatCountdown(30)).toBe("30s");
    expect(formatCountdown(59)).toBe("59s");
  });

  it("returns minutes and seconds when 60 or more", () => {
    expect(formatCountdown(60)).toBe("1m 0s");
    expect(formatCountdown(90)).toBe("1m 30s");
    expect(formatCountdown(125)).toBe("2m 5s");
    expect(formatCountdown(3661)).toBe("61m 1s");
  });
});
