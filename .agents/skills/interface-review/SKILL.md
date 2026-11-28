---
name: interface-review
description: Review a Codync UI change for hierarchy, motion, accessibility and cross-platform parity. Use when designing, reviewing or polishing a screen in Desktop, iOS, Android or the terminal UI.
---

# Interface review

Codync's UI rules live in `AGENTS.md`, `docs/design/ui-conventions.md` and
`docs/guides/desktop-styles.md`. Read them before judging a screen. This skill is the checklist
that turns those rules into a pass/fail review.

## 1. Behavior before decoration

- State what the screen lets a person do, then check each action has a visible result.
- Loading, empty, error and permission states must exist for every async surface.
- No dead controls, no swallowed errors, no placeholders that look finished.

## 2. Shared components

- Custom controls first; the native exceptions are named in `AGENTS.md` (iOS navigation and
  `TabView`, desktop menu bar, system auth and widget containers).
- A surface with a background fill gets no border line.
- Every tap that shows or hides something animates through `Motion` (Swift/Kotlin) or the CSS
  transition tokens (Desktop).

## 3. Hierarchy and copy

- One primary action per surface; secondary actions are visibly quieter.
- Labels say what happens, not what the code does. Keep the same words across the four clients.
- Trim copy that repeats the header or the button directly below it.

## 4. Accessibility

- Every control has an accessible name; role and state match the visual state.
- Text survives large type and localization (German compounds and CJK are the canaries).
- Reduced motion removes decorative animation, never the state change.

## 5. Cross-platform parity

- A UI change lands in Desktop, iOS, Android and the terminal UI in the same change, or the
  summary names the clients left out and why.
- Compare layouts: adapt controls and input, preserve the same user-facing behavior.
- Touch targets, keyboard focus and back navigation must work on each platform.

## 6. Evidence

Name the checks actually run (typecheck, unit tests, emulator or browser pass). If a check could
not run, say why. Do not call a design reviewed without looking at the rendered result.
