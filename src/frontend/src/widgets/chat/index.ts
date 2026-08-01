// ChatWidget is deliberately NOT re-exported here. It is lazy-loaded in widgetMap.ts, which
// imports the three eager widgets below from this barrel - re-exporting ChatWidget would give the
// eager graph a static edge to it and silently defeat the code-split (the build stays green and the
// chunk still looks normal). See "Module Graph and Lazy Loading" in README.md.
export * from "./ChatMessageWidget";
export * from "./ChatLoadingWidget";
export * from "./ChatStatusWidget";
