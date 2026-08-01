import { render, waitFor } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import Icon from "./Icon";

describe("Icon", () => {
  it("renders None icon synchronously", () => {
    const { container } = render(<Icon name="None" />);
    const svg = container.querySelector("svg");
    expect(svg).toBeInTheDocument();
    expect(svg).toHaveClass("invisible");
  });

  it("renders IvyCorner icon synchronously", () => {
    const { container } = render(<Icon name="IvyCorner" />);
    const svg = container.querySelector("svg");
    expect(svg).toBeInTheDocument();
    expect(svg).toHaveAttribute("viewBox", "0 0 12 12");
  });

  it("renders react-icons synchronously", () => {
    const { container } = render(<Icon name="Google" />);
    const svg = container.querySelector("svg");
    expect(svg).toBeInTheDocument();
  });

  it("renders lucide icons asynchronously", async () => {
    const { container } = render(<Icon name="Home" />);
    await waitFor(() => {
      const svg = container.querySelector("svg");
      expect(svg).toBeInTheDocument();
    });
  });

  it("renders nothing for unknown icon name", async () => {
    const { container } = render(<Icon name="NotAnIcon" />);
    await waitFor(
      () => {
        const svg = container.querySelector("svg");
        expect(svg).not.toBeInTheDocument();
      },
      { timeout: 100 }
    );
  });
});
