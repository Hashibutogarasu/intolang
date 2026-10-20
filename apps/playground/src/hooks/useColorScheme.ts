import { useEffect, useState } from "react";

/** Whether the page is shown in a dark colour scheme, following the system. */
export function useDarkScheme() {
  const [dark, setDark] = useState(false);
  useEffect(() => {
    const scheme = window.matchMedia("(prefers-color-scheme: dark)");
    const follow = () => setDark(scheme.matches);
    follow();
    scheme.addEventListener("change", follow);
    return () => scheme.removeEventListener("change", follow);
  }, []);
  return dark;
}
