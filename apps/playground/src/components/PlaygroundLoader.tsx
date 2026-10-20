"use client";

import dynamic from "next/dynamic";

import Overlay from "./Overlay";

const Playground = dynamic(() => import("./Playground"), {
  ssr: false,
  loading: () => <Overlay visible />,
});

export default function PlaygroundLoader() {
  return <Playground />;
}
