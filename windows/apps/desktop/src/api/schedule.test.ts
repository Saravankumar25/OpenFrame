import { describe, expect, it } from "vitest";
import { callSheetStatusWord, formatClock, formatMinutes, formatMoney, formatPages, parseClock, parseDuration, parseMoney, parsePages, minorToInput } from "./schedule";
import { monthGrid, shiftMonth } from "../workspaces/call-sheets/shared/calendar";

describe("schedule formatting", () => {
  it("formats and parses durations without inventing values", () => {
    expect(formatMinutes(330)).toBe("5h 30m");
    expect(formatMinutes(600)).toBe("10h");
    expect(formatMinutes(45)).toBe("45m");
    expect(parseDuration("")).toBeNull();
    expect(parseDuration("90")).toBe(90);
    expect(parseDuration("1:30")).toBe(90);
    expect(parseDuration("1h 30m")).toBe(90);
    expect(parseDuration("1.5h")).toBe(90);
    expect(parseDuration("45m")).toBe(45);
    expect(parseDuration("soon")).toBeNaN();
  });

  it("formats and parses page eighths", () => {
    expect(formatPages(10)).toBe("1 2/8");
    expect(formatPages(3)).toBe("3/8");
    expect(formatPages(16)).toBe("2");
    expect(parsePages("1 2/8")).toBe(10);
    expect(parsePages("3/8")).toBe(3);
    expect(parsePages("2")).toBe(16);
    expect(parsePages("")).toBeNull();
    expect(parsePages("two")).toBeNaN();
  });

  it("parses money into minor units and rejects negatives", () => {
    expect(parseMoney("12,00,000", "INR")).toBe(120_000_000);
    expect(parseMoney("1500.50", "USD")).toBe(150_050);
    expect(parseMoney("₹ 2,000", "INR")).toBe(200_000);
    expect(parseMoney("1000", "JPY")).toBe(1000);
    expect(parseMoney("", "USD")).toBeNull();
    expect(parseMoney("   ", "USD")).toBeNull();
    expect(parseMoney("abc", "USD")).toBeNaN();
    expect(parseMoney("₹", "INR")).toBeNaN();
    expect(parseMoney("-5", "USD")).toBeNaN();
    expect(parseMoney("1.234", "USD")).toBeNaN();
    expect(minorToInput(150_050, "USD")).toBe("1500.50");
    expect(minorToInput(120_000_000, "INR")).toBe("1200000");
    expect(formatMoney(120_000_000, "INR")).toContain("12,00,000");
  });
});

describe("day times", () => {
  it("parses typed clock times and formats night wraps", () => {
    expect(parseClock("7")).toBe(420);
    expect(parseClock("07:30")).toBe(450);
    expect(parseClock("730")).toBe(450);
    expect(parseClock("19.15")).toBe(1155);
    expect(parseClock("")).toBeNull();
    expect(parseClock("24:00")).toBeNaN();
    expect(parseClock("abc")).toBeNaN();
    expect(formatClock(420)).toBe("07:00");
    expect(formatClock(1440 + 90)).toBe("01:30 (+1)");
  });

  it("uses the owner's three call sheet status words", () => {
    expect(callSheetStatusWord("Draft")).toBe("Draft");
    expect(callSheetStatusWord("Needs refresh")).toBe("Needs refresh");
    expect(callSheetStatusWord("Final")).toBe("Final");
    expect(callSheetStatusWord("")).toBe("Draft");
  });
});

describe("calendar grid", () => {
  it("builds six Monday-first weeks", () => {
    const g = monthGrid(2027, 5); // June 2027 starts on a Tuesday
    expect(g).toHaveLength(42);
    expect(g[0].iso).toBe("2027-05-31");
    expect(g[1]).toEqual({ iso: "2027-06-01", day: 1, inMonth: true });
    expect(g.filter((c) => c.inMonth)).toHaveLength(30);
  });

  it("shifts months across years", () => {
    expect(shiftMonth(2027, 0, -1)).toEqual([2026, 11]);
    expect(shiftMonth(2027, 11, 1)).toEqual([2028, 0]);
  });
});
