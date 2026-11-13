import "svelte/elements";

declare module "svelte/elements" {
  interface HTMLAttributes<T> {
    /** Fired on the focused element when the "options" action (X by default) is pressed. */
    onnavoptions?: (e: CustomEvent) => void;
  }
}
export {};
