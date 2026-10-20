import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";

type Props = {
  visible: boolean;
  /** How long the opacity takes to change, in milliseconds. */
  fadeMs?: number;
};

/**
 * A cover of a colour that does not follow the theme, with a circular
 * progress in the middle. Whatever is under it can change unseen; only its
 * opacity changes in the sight of the user.
 */
export default function Overlay({ visible, fadeMs = 300 }: Props) {
  return (
    <div
      aria-hidden={!visible}
      style={{ transitionDuration: `${fadeMs}ms` }}
      className={cn(
        "fixed inset-0 z-50 grid place-items-center bg-zinc-900 transition-opacity motion-reduce:transition-none",
        visible ? "opacity-100" : "pointer-events-none opacity-0",
      )}
    >
      <Spinner className="size-10 text-zinc-300" />
    </div>
  );
}
