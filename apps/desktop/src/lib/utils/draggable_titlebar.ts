import { getCurrentWindow } from '@tauri-apps/api/window';

/**
 * Svelte action: makes the host element a draggable titlebar region.
 *
 * Calls `getCurrentWindow().startDragging()` on mousedown when the click
 * target is NOT an interactive element. This is the manual equivalent of
 * `data-tauri-drag-region`, but with one critical difference: the
 * attribute-based approach disables clicks on every child element of the
 * region (buttons, links, inputs all stop working), whereas this action
 * only intercepts mousedown when the actual target is non-interactive,
 * so children like buttons, links, and form controls keep working.
 *
 * Place this on a wrapper element. The wrapper itself is the fallback
 * drag handle; child buttons/links still receive their own clicks.
 */
export function draggable_titlebar(node: HTMLElement) {
  const is_interactive = (el: EventTarget | null): boolean => {
    let cur: HTMLElement | null = el as HTMLElement | null;
    while (cur && cur !== node) {
      if (cur instanceof HTMLButtonElement) return true;
      if (cur instanceof HTMLAnchorElement) return true;
      if (cur instanceof HTMLInputElement) return true;
      if (cur instanceof HTMLTextAreaElement) return true;
      if (cur instanceof HTMLSelectElement) return true;
      if (cur.isContentEditable) return true;
      if (cur.dataset?.noDrag === '') return true;
      cur = cur.parentElement;
    }
    return false;
  };

  const on_mouse_down = (e: MouseEvent) => {
    // Primary button only. Anything else is right-click/aux.
    if (e.button !== 0) return;
    if (is_interactive(e.target)) return;
    // Prevent the webview from interpreting the gesture as a selection or
    // click on the underlying element.
    e.preventDefault();
    void getCurrentWindow().startDragging();
  };

  // Also stop the default browser drag-image/selection on the region.
  node.style.setProperty('-webkit-user-select', 'none');
  node.style.setProperty('user-select', 'none');

  node.addEventListener('mousedown', on_mouse_down, true);
  return {
    destroy() {
      node.removeEventListener('mousedown', on_mouse_down, true);
    },
  };
}
