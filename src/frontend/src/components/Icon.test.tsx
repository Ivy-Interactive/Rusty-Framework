import { describe, it, expect, beforeEach, afterEach } from "vitest";
import React, { act } from "react";
import { createRoot, Root } from "react-dom/client";
import Icon from "./Icon";

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

describe("Icon", () => {
  it("renders None icon synchronously", () => {
    mount(<Icon name="None" />);
    const svg = container.querySelector("svg");
    expect(svg).not.toBeNull();
    expect(svg!.classList.contains("invisible")).toBe(true);
  });

  it("renders IvyCorner icon synchronously", () => {
    mount(<Icon name="IvyCorner" />);
    const svg = container.querySelector("svg");
    expect(svg).not.toBeNull();
    expect(svg!.getAttribute("viewBox")).toBe("0 0 12 12");
  });

  it("renders react-icons synchronously", () => {
    mount(<Icon name="Google" />);
    const svg = container.querySelector("svg");
    expect(svg).not.toBeNull();
  });

  it("renders lucide icons asynchronously", async () => {
    mount(<Icon name="Home" />);
    await new Promise((resolve) => setTimeout(resolve, 100));
    const svg = container.querySelector("svg");
    expect(svg).not.toBeNull();
  });

  it("renders nothing for unknown icon name", async () => {
    mount(<Icon name="NotAnIcon" />);
    await new Promise((resolve) => setTimeout(resolve, 100));
    const svg = container.querySelector("svg");
    expect(svg).toBeNull();
  });
});
