// Players for recorded media in the Idea Vault drawer.

/** A recording made in the app (MediaRecorder, WebM) carries no length in its header, so the
 * player reports an endless duration: no total time and no seeking until it has played through.
 * Asking for a position far past the end makes the webview read to the end and learn the length;
 * the player then goes back to the start. Files that know their length are left alone. */
export function settleUnknownDuration(el: HTMLMediaElement): void {
  if (el.duration !== Infinity) return;
  const back = () => {
    if (!Number.isFinite(el.duration)) return;
    el.removeEventListener("durationchange", back);
    el.currentTime = 0;
  };
  el.addEventListener("durationchange", back);
  el.currentTime = Number.MAX_SAFE_INTEGER;
}
