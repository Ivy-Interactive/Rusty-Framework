import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { flushSync } from "react-dom";
import { createRoot, Root } from "react-dom/client";
import LucideIcon from "./LucideIcon";

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  root.unmount();
  container.remove();
});

describe("LucideIcon", () => {
  it("renders an svg for a known icon name", () => {
    flushSync(() => {
      root.render(<LucideIcon name="Folder" />);
    });
    const svg = container.querySelector("svg");
    expect(svg).not.toBeNull();
  });

  it("renders nothing for an unknown icon name", () => {
    flushSync(() => {
      root.render(<LucideIcon name="NotAnIcon" />);
    });
    const svg = container.querySelector("svg");
    expect(svg).toBeNull();
  });

  it("passes size and className through to the icon component", () => {
    flushSync(() => {
      root.render(<LucideIcon name="Folder" size={32} className="test-class" />);
    });
    const svg = container.querySelector("svg");
    expect(svg).not.toBeNull();
    expect(svg!.getAttribute("width")).toBe("32");
    expect(svg!.getAttribute("height")).toBe("32");
    expect(svg!.classList.contains("test-class")).toBe(true);
  });
});
