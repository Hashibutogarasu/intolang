"use client";

import dynamic from "next/dynamic";

const Playground = dynamic(() => import("./Playground"), {
  ssr: false,
  loading: () => <p className="loading">Loading…</p>,
});

export default function PlaygroundLoader() {
  return <Playground />;
}
