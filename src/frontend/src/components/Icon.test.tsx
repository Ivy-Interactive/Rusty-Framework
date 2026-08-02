import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { flushSync } from "react-dom";
import { createRoot, Root } from "react-dom/client";
import Icon from "./Icon";

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

describe("Icon", () => {
  it("renders None icon synchronously", () => {
    flushSync(() => {
      root.render(<Icon name="None" />);
    });
    const svg = container.querySelector("svg");
    expect(svg).not.toBeNull();
    expect(svg!.classList.contains("invisible")).toBe(true);
  });

  it("renders IvyCorner icon synchronously", () => {
    flushSync(() => {
      root.render(<Icon name="IvyCorner" />);
    });
    const svg = container.querySelector("svg");
    expect(svg).not.toBeNull();
    expect(svg!.getAttribute("viewBox")).toBe("0 0 12 12");
  });

  it("renders react-icons synchronously", () => {
    flushSync(() => {
      root.render(<Icon name="Google" />);
    });
    const svg = container.querySelector("svg");
    expect(svg).not.toBeNull();
  });

  it("renders Suspense fallback for lucide icons", () => {
    flushSync(() => {
      root.render(<Icon name="Home" />);
    });
    const svg = container.querySelector("svg");
    expect(svg).toBeNull();
  });

  it("renders Suspense fallback for unknown icon name", () => {
    flushSync(() => {
      root.render(<Icon name="NotAnIcon" />);
    });
    const svg = container.querySelector("svg");
    expect(svg).toBeNull();
  });
});
