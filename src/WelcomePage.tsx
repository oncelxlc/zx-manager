import * as stylex from "@stylexjs/stylex";
import { ChevronRight } from "lucide-react";
import { useDispatch, useSelector } from "react-redux";
import { Link } from "react-router";
import { Badge } from "./components/ui/badge";
import { toggleDependencies, type RootState } from "./store";

const dependencies = [
  "Tauri", "React", "Vite", "React Router", "TypeScript",
  "StyleX", "lucide-react", "shadcn/ui", "Redux Toolkit",
];

export function WelcomePage() {
  const expanded = useSelector((state: RootState) => state.welcome.dependenciesExpanded);
  const dispatch = useDispatch();

  return (
    <main {...stylex.props(styles.page)}>
      <div {...stylex.props(styles.content)}>
        <img src="/icon.png" width={50} height={50} alt="" />
        <h1 {...stylex.props(styles.heading)}>
          <span {...stylex.props(styles.greeting)}>欢迎使用</span>{" "}
          <span {...stylex.props(styles.name)}>zx-once</span>
        </h1>
        <Link to="/settings/island" {...stylex.props(styles.settings)}>灵动岛设置</Link>
        <section aria-labelledby="dependencies" {...stylex.props(styles.section)}>
          <h2 id="dependencies" {...stylex.props(styles.sectionHeading)}>
            <button
              type="button"
              aria-expanded={expanded}
              aria-controls="dependency-list"
              onClick={() => dispatch(toggleDependencies())}
              {...stylex.props(styles.toggle)}
            >
              主要依赖
              <ChevronRight size={16} aria-hidden="true" {...stylex.props(expanded && styles.expanded)} />
            </button>
          </h2>
          <div id="dependency-list" hidden={!expanded}>
            <ul {...stylex.props(styles.list)}>
              {dependencies.map((name) => (
                <li key={name}>
                  <Badge variant="outline">{name}</Badge>
                </li>
              ))}
            </ul>
          </div>
        </section>
      </div>
    </main>
  );
}

const styles = stylex.create({
  page: {
    minHeight: "100svh",
    display: "grid",
    justifyItems: "center",
    alignItems: "start",
    paddingBlockStart: "clamp(3rem, 28svh, 18rem)",
    paddingBlockEnd: "3rem",
    paddingInline: "1.5rem",
    backgroundColor: "#fafaf9",
    color: "#1c1917",
  },
  content: { width: "100%", maxWidth: "36rem", minWidth: 0 },
  heading: { margin: "1.5rem 0 0", fontWeight: 600 },
  greeting: { display: "block", fontSize: "1.25rem", color: "#57534e" },
  name: {
    display: "block",
    marginTop: "0.25rem",
    fontSize: "clamp(2.5rem, 8vw, 4.5rem)",
    letterSpacing: "-0.045em",
    lineHeight: 1.08,
  },
  section: { marginTop: "2.5rem", paddingTop: "1rem", borderTop: "1px solid #d6d3d1" },
  settings: {
    display: "inline-flex", alignItems: "center", minHeight: "2.75rem", marginTop: "1rem",
    color: "#16685a", fontWeight: 600, textUnderlineOffset: "0.25rem",
  },
  sectionHeading: { margin: 0 },
  toggle: {
    display: "inline-flex",
    alignItems: "center",
    gap: "0.375rem",
    minHeight: "2.75rem",
    padding: 0,
    borderWidth: 0,
    backgroundColor: "transparent",
    color: { default: "#57534e", ":hover": "#1c1917" },
    fontFamily: "inherit",
    fontSize: "0.875rem",
    fontWeight: 600,
    cursor: "pointer",
    outlineOffset: "0.25rem",
  },
  expanded: { transform: "rotate(90deg)" },
  list: {
    display: "flex",
    flexWrap: "wrap",
    gap: "0.5rem",
    margin: "0.25rem 0 0",
    padding: 0,
    listStyle: "none",
  },
});
