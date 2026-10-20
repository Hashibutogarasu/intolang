import { useEffect, useState } from "react";

/**
 * Whether the page is shown in a dark colour scheme, following the system,
 * and whether the page is `covered` while it changes. The `dark` class of the
 * page follows the scheme too, for the theme of shadcn.
 *
 * A change first asks for the cover, which fades in over `fadeMs`, and only
 * then switches the theme, so that nothing of the switch can be seen. The
 * page is uncovered once the new theme has been drawn. The page changes at
 * once when the user asks for less motion.
 */
export function useColorScheme(fadeMs = 300) {
  const [dark, setDark] = useState(false);
  const [covered, setCovered] = useState(false);

  useEffect(() => {
    const page = document.documentElement;
    const scheme = window.matchMedia("(prefers-color-scheme: dark)");
    const lessMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
    let waiting: ReturnType<typeof setTimeout> | undefined;

    const apply = () => {
      setDark(scheme.matches);
      page.classList.toggle("dark", scheme.matches);
    };
    const change = () => {
      if (lessMotion.matches) {
        apply();
        return;
      }
      setCovered(true);
      clearTimeout(waiting);
      waiting = setTimeout(() => {
        apply();
        requestAnimationFrame(() => requestAnimationFrame(() => setCovered(false)));
      }, fadeMs);
    };

    apply();
    scheme.addEventListener("change", change);
    return () => {
      clearTimeout(waiting);
      scheme.removeEventListener("change", change);
    };
  }, [fadeMs]);

  return { dark, covered, fadeMs };
}
