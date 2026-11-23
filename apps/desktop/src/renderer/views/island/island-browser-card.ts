/**
 * The island's browser-card copy (OpenBot 0.77.0). The host owns the decision and the card data;
 * this only says what Open does and that an answer still reaches the agent after its wait ends.
 * No URL or page content is read here.
 */

const PAGE_STEP = /browser|web ?page|sign[ -]?in|captcha|takeover|web site|website/i

/** A one-line hint under the card title, or null for a card that has nothing to do with a page. */
export function browserCardSubtitle(
  headline: string,
  title: string,
  options: { label: string }[],
): string | null {
  const pageStep = PAGE_STEP.test(headline) || PAGE_STEP.test(title)
  if (options.some((option) => option.label.trim().toLowerCase() === 'open')) {
    return 'Open shows the page in the main window, where you can type.'
  }
  return pageStep ? 'Your answer still goes to the agent, even if it stopped waiting.' : null
}
