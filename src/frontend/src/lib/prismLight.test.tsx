import { describe, it, expect } from "vitest";
import { createRoot } from "react-dom/client";
import { act } from "react";
import SyntaxHighlighter, { REGISTERED_LANGUAGES } from "./prismLight";

describe("prismLight", () => {
  const samples: Record<string, string> = {
    bash: 'echo "hello"',
    markdown: "# Title\n\nParagraph",
    markup: "<div>hello</div>",
    yaml: "key: value",
    json: '{"key": "value"}',
    python: "def func():\n    pass",
    rust: "fn main() {}",
    go: "func main() {}",
    java: "public class Main {}",
    sql: "SELECT * FROM users;",
    css: "body { margin: 0; }",
    javascript: "const x = 42;",
    typescript: "const x: number = 42;",
    diff: "+added line\n-removed line",
    ini: "[section]\nkey=value",
    makefile: "target:\n\tcommand",
  };

  describe.each(REGISTERED_LANGUAGES)("registers %s", (language) => {
    it(`renders ${language} with syntax highlighting`, () => {
      const container = document.createElement("div");
      const root = createRoot(container);
      const code = samples[language] ?? 'var s = "hi";';

      act(() => {
        root.render(
          <SyntaxHighlighter language={language} style={{}}>
            {code}
          </SyntaxHighlighter>,
        );
      });

      const tokens = container.querySelectorAll("span.token");
      expect(tokens.length).toBeGreaterThan(0);

      act(() => {
        root.unmount();
      });
    });
  });

  it("renders markup aliases (xml) with syntax highlighting", () => {
    const container = document.createElement("div");
    const root = createRoot(container);

    act(() => {
      root.render(
        <SyntaxHighlighter language="xml" style={{}}>
          {"<root><child /></root>"}
        </SyntaxHighlighter>,
      );
    });

    const tokens = container.querySelectorAll("span.token");
    expect(tokens.length).toBeGreaterThan(0);

    act(() => {
      root.unmount();
    });
  });

  it("renders unregistered languages as plain text", () => {
    const container = document.createElement("div");
    const root = createRoot(container);
    const code = "+++[>+++<-]";

    act(() => {
      root.render(
        <SyntaxHighlighter language="brainfuck" style={{}}>
          {code}
        </SyntaxHighlighter>,
      );
    });

    const tokens = container.querySelectorAll("span.token");
    expect(tokens.length).toBe(0);
    expect(container.textContent).toContain(code);

    act(() => {
      root.unmount();
    });
  });
});
