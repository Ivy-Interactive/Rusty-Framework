import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import LucideIcon from "./LucideIcon";

describe("LucideIcon", () => {
  it("renders an svg for a known icon name", () => {
    const { container } = render(<LucideIcon name="Folder" />);
    const svg = container.querySelector("svg");
    expect(svg).toBeInTheDocument();
  });

  it("renders nothing for an unknown icon name", () => {
    const { container } = render(<LucideIcon name="NotAnIcon" />);
    const svg = container.querySelector("svg");
    expect(svg).not.toBeInTheDocument();
  });

  it("passes props through to the icon component", () => {
    const { container } = render(
      <LucideIcon name="Folder" color="red" size={32} className="test-class" />
    );
    const svg = container.querySelector("svg");
    expect(svg).toHaveAttribute("color", "red");
    expect(svg).toHaveAttribute("width", "32");
    expect(svg).toHaveAttribute("height", "32");
    expect(svg).toHaveClass("test-class");
  });
});
