/**
 * The Threads page needs a fill behind the composer only when there is something to hide:
 * threads scroll under it and the window is transparent (or shows a backdrop). Otherwise the
 * composer sits on the page's own surface and the fill would show as a dark rectangle.
 */
export function composer_fill(scrolled: boolean, transparent: boolean): boolean {
  return scrolled === true && transparent === true
}
