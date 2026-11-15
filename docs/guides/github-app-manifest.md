# Setting up a GitHub App

A GitHub App lets a routine run on pull requests, reviews, checks and issues without a person's
token. It is created once per computer and installed only on the repositories that should reach
bots.

## Creating the App

1. Open <https://github.com/settings/apps/new> (an organization's **Settings → Developer settings →
   GitHub Apps → New GitHub App** works too, so the installation can cover its repositories).
2. **Name** it; its slug fills in by itself.
3. **Homepage URL:** the repository or site this App is for.
4. **Webhook URL:** the relay's public hook of the computer, `https://<relay host>/hooks/github`.
   A private relay serves the same path.
5. **Webhook secret:** generate one with at least 16 visible characters and keep it; it is pasted
   into Codync and checked against every delivery.
6. Under **Repository permissions** give **Contents: Read**, **Pull requests: Read** and
   **Checks: Read**.
7. Under **Subscribe to events** tick **Pull request**, **Pull request review**, **Check run** and
   **Issue**.
8. **Create GitHub App**, then generate a **private key** and download the PEM file.

## Connecting it

On the computer, open **Settings → GitHub App** and paste the App id (from the App's page), the PEM
file's text and the webhook secret. The host checks them and reads the App's installations before
saving.

Then install the App on the account or organization and choose its repositories (**Install on
repositories**), and press **Refresh installations** back in Codync. For each repository, **Map
bots** picks which bots its events reach, and can limit it to branches.

GitHub's ping arrives as soon as the URL is saved; Codync answers `{"ping": true}`.

## What reaches a bot

Only routines with an event trigger of this source run, and only on the mapped bots and branches.
The four subscribed events map to the trigger's event name:
`pull_request.opened`, `pull_request_review.submitted`, `check_run.completed` and `issues.closed`,
among others; the editor offers the catalog.

## Checking it

Deliveries are listed in the App's **Advanced** page on GitHub. A `202` with `{"queued": 1}` means a
routine was queued; `{"ignored": …}` says why nothing was (another event, an unmapped repository or
branch), and `401` means the signature did not match the stored secret. Deliveries are remembered
for ten minutes, so a resend inside that window answers `{"duplicate": true}`.
