"use client";

import dynamic from "next/dynamic";

import Loading from "./Loading";

const Playground = dynamic(() => import("./Playground"), {
  ssr: false,
  loading: Loading,
});

export default function PlaygroundLoader() {
  return <Playground />;
}
