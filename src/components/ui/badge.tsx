import { mergeProps } from "@base-ui/react/merge-props";
import { useRender } from "@base-ui/react/use-render";
import * as stylex from "@stylexjs/stylex";

const styles = stylex.create({
  badge: {
    display: "inline-flex",
    alignItems: "center",
    borderRadius: "0.5rem",
    paddingBlock: "0.25rem",
    paddingInline: "0.625rem",
    fontSize: "0.875rem",
    fontWeight: 500,
    lineHeight: 1.5,
    whiteSpace: "nowrap",
  },
  default: {
    backgroundColor: "#292524",
    color: "#fff",
  },
  outline: {
    backgroundColor: "#fff",
    borderColor: "#d6d3d1",
    borderStyle: "solid",
    borderWidth: 1,
    color: "#292524",
  },
});

// Adapted from the shadcn/ui Base UI Badge.
export function Badge({
  variant = "default",
  render,
  ...props
}: useRender.ComponentProps<"span"> & { variant?: "default" | "outline" }) {
  return useRender({
    defaultTagName: "span",
    props: mergeProps<"span">(stylex.props(styles.badge, styles[variant]), props),
    render,
    state: { slot: "badge", variant },
  });
}
