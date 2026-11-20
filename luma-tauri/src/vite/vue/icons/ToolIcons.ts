import { h, type Component } from "vue";

type Shape =
  | { d: string }
  | { cx: number; cy: number; r: number };

/** Small 24x24 stroke icon as a functional component (inherits currentColor). */
export function strokeIcon(shapes: Shape[]): Component {
  return () =>
    h(
      "svg",
      {
        viewBox: "0 0 24 24",
        width: 18,
        height: 18,
        fill: "none",
        stroke: "currentColor",
        "stroke-width": 1.8,
        "stroke-linecap": "round",
        "stroke-linejoin": "round",
        "aria-hidden": "true",
      },
      shapes.map((shape) =>
        "d" in shape
          ? h("path", { d: shape.d })
          : h("circle", { cx: shape.cx, cy: shape.cy, r: shape.r })
      )
    );
}

export const FolderIcon = strokeIcon([
  { d: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" },
]);

export const CommitIcon = strokeIcon([
  { cx: 12, cy: 12, r: 3.2 },
  { d: "M3 12h5.8M15.2 12H21" },
]);

export const TerminalIcon = strokeIcon([
  { d: "M4 17l6-5-6-5" },
  { d: "M12 19h8" },
]);

export const AnalyzerIcon = strokeIcon([
  { d: "M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z" },
  { d: "M12 9v4" },
  { d: "M12 17h.01" },
]);
