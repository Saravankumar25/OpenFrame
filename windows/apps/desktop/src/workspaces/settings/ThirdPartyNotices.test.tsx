import { describe, expect, it } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { ThirdPartyNotices } from "./ThirdPartyNotices";
import notices from "./third-party-notices.json";
import lock from "../../../../../packaging/ai/ai-assets.lock.json";

describe("Third-party notices", () => {
  it("shows the Gemma notice and use restrictions before anything is opened", () => {
    render(<ThirdPartyNotices />);
    expect(
      screen.getByText(/Gemma is provided under and subject to the Gemma Terms of Use found at ai\.google\.dev\/gemma\/terms\./),
    ).toBeInTheDocument();
    expect(screen.getByText(/Gemma Prohibited Use Policy/)).toBeInTheDocument();
  });

  it("lists the engine, every model, the fonts and every library, and opens a full text", async () => {
    render(<ThirdPartyNotices />);
    fireEvent.click(screen.getByRole("button", { name: "Show licences and notices" }));
    expect(await screen.findByText(/^llama\.cpp \(the Straft AI engine/)).toBeInTheDocument();
    for (const m of lock.models) expect(screen.getByText(new RegExp(`^${m.displayName}`))).toBeInTheDocument();
    expect(screen.getByText(/Courier Prime \(OFL-1\.1\)/)).toBeInTheDocument();
    expect(screen.getByText(`Libraries (${notices.packages.length})`)).toBeInTheDocument();
    const details = screen.getByText(/^tauri \d/).closest("details")!;
    details.open = true;
    fireEvent(details, new Event("toggle"));
    expect(await within(details).findByText(/Apache License|MIT License|Permission is hereby granted/)).toBeInTheDocument();
  });

  it("has a text for nearly every library, and every referenced text exists", () => {
    const withText = notices.packages.filter((p) => p.texts.length).length;
    expect(withText / notices.packages.length).toBeGreaterThan(0.95);
    for (const p of notices.packages) for (const t of p.texts) expect(notices.texts).toHaveProperty(t);
  });
});
