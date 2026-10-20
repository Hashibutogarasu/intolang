import { Spinner } from "@/components/ui/spinner";

/** A circular progress in the middle of the space it is given. */
export default function Loading() {
  return (
    <div className="grid size-full min-h-screen place-items-center">
      <Spinner className="size-10 text-muted-foreground" />
    </div>
  );
}
