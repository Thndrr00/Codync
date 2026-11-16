"use client";

import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";

export type Theme = "system" | "light" | "dark";

const STORAGE_KEY = "codync.web.theme";
const DARK = "#0a0a0a";
const LIGHT = "#fafafa";

type ThemeValue = {
  theme: Theme;
  /** What the page shows now: system already resolved. */
  resolved: "light" | "dark";
  setTheme: (theme: Theme) => void;
};

const ThemeContext = createContext<ThemeValue | null>(null);

function stored(): Theme {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    return value === "light" || value === "dark" || value === "system" ? value : "system";
  } catch {
    return "system";
  }
}

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const [theme, setThemeState] = useState<Theme>("system");
  // The site is dark, so the first paint matches it before the system preference is known.
  const [systemDark, setSystemDark] = useState(true);

  useEffect(() => {
    setThemeState(stored());
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    setSystemDark(query.matches);
    const changed = (event: MediaQueryListEvent) => setSystemDark(event.matches);
    query.addEventListener("change", changed);
    return () => query.removeEventListener("change", changed);
  }, []);

  const resolved: "light" | "dark" = theme === "system" ? (systemDark ? "dark" : "light") : theme;

  useEffect(() => {
    document.documentElement.dataset.appTheme = resolved;
    const previous = document.body.style.backgroundColor;
    document.body.style.backgroundColor = resolved === "dark" ? DARK : LIGHT;
    document.querySelector('meta[name="theme-color"]')?.setAttribute("content", resolved === "dark" ? DARK : LIGHT);
    return () => {
      document.body.style.backgroundColor = previous;
    };
  }, [resolved]);

  const setTheme = useCallback((next: Theme) => {
    setThemeState(next);
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // A private window can refuse storage; the choice still holds for this page.
    }
  }, []);

  const value = useMemo(() => ({ theme, resolved, setTheme }), [theme, resolved, setTheme]);
  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeValue {
  const value = useContext(ThemeContext);
  if (!value) throw new Error("useTheme must be used inside ThemeProvider");
  return value;
}
