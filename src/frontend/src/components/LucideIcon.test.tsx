import { describe, it, expect, beforeEach, afterEach } from "vitest";
import React, { act } from "react";
import { createRoot, Root } from "react-dom/client";
import LucideIcon from "./LucideIcon";

let container: HTMLDivElement;
let root: Root;

function mount(element: React.ReactElement) {
  act(() => {
    root.render(element);
  });
}

beforeEach(() => {
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => {
    root.unmount();
  });
  container.remove();
});

describe("LucideIcon", () => {
  it("renders an svg for a known icon name", () => {
    mount(<LucideIcon name="Folder" />);
    const svg = container.querySelector("svg");
    expect(svg).not.toBeNull();
  });

  it("renders nothing for an unknown icon name", () => {
    mount(<LucideIcon name="NotAnIcon" />);
    const svg = container.querySelector("svg");
    expect(svg).toBeNull();
  });

  it("passes props through to the icon component", () => {
    mount(<LucideIcon name="Folder" color="red" size={32} className="test-class" />);
    const svg = container.querySelector("svg");
    expect(svg).not.toBeNull();
    expect(svg!.getAttribute("color")).toBe("red");
    expect(svg!.getAttribute("width")).toBe("32");
    expect(svg!.getAttribute("height")).toBe("32");
    expect(svg!.classList.contains("test-class")).toBe(true);
  });
});
