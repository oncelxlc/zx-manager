import { createContext, useContext, useEffect, useState } from "react";
import type { ReactNode } from "react";
import { setTheme as persistTheme } from "src/services/storage/preferences-storage";
import type { ResolvedTheme, ThemeMode } from "src/types/preferences";

type ThemeProviderProps = {
  children: ReactNode
  defaultTheme?: ThemeMode
}

type ThemeProviderState = {
  theme: ThemeMode
  resolvedTheme: ResolvedTheme
  setTheme: (theme: ThemeMode) => void
}

const initialState: ThemeProviderState = {
  theme: "system",
  resolvedTheme: "light",
  setTheme: () => null,
};

const ThemeProviderContext = createContext<ThemeProviderState>(initialState);

export function ThemeProvider({
                                children,
                                defaultTheme = "dark",
                                ...props
                              }: ThemeProviderProps) {
  const [theme, setThemeState] = useState<ThemeMode>(defaultTheme);
  const [resolvedTheme, setResolvedTheme] = useState<ResolvedTheme>(() =>
    resolveTheme(defaultTheme),
  );

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }

    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const updateResolvedTheme = () => {
      const nextTheme = resolveTheme(theme, mediaQuery.matches);
      setResolvedTheme(nextTheme);
      applyThemeClass(nextTheme);
    };

    updateResolvedTheme();
    if (theme !== "system") {
      return;
    }

    mediaQuery.addEventListener("change", updateResolvedTheme);
    return () => mediaQuery.removeEventListener("change", updateResolvedTheme);
  }, [theme]);

  const value = {
    theme,
    resolvedTheme,
    setTheme: (nextTheme: ThemeMode) => {
      setThemeState(nextTheme);
      void persistTheme(nextTheme);
    },
  };

  return (
    <ThemeProviderContext.Provider {...props} value={value}>
      {children}
    </ThemeProviderContext.Provider>
  );
}

export const useTheme = () => {
  const context = useContext(ThemeProviderContext);

  if (context === undefined)
    throw new Error("useTheme must be used within a ThemeProvider");

  return context;
};

function resolveTheme(theme: ThemeMode, systemIsDark?: boolean): ResolvedTheme {
  if (theme !== "system") {
    return theme;
  }

  const isDark = systemIsDark ??
    (typeof window !== "undefined" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  return isDark ? "dark" : "light";
}

function applyThemeClass(theme: ResolvedTheme) {
  if (typeof document === "undefined") {
    return;
  }

  document.documentElement.classList.remove("light", "dark");
  document.documentElement.classList.add(theme);
}

export function applyTheme(theme: ThemeMode) {
  applyThemeClass(resolveTheme(theme));
}
