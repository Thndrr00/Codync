# GitHub App

A [routine](routines.md) can run when a GitHub event arrives. Without an App, that needs a
person's token, which is broad and belongs to an account. A GitHub App instead signs each delivery
with its own key, is installed only on the repositories the user picks, and mints short-lived
installation tokens on demand. The idea comes from agentes.chat as a pattern, not ported code.

The host owns the App end to end: registration, the vault entries, the webhook, the deduplication,
the repository-to-bot mapping and the tokens. Clients render that state; only the desktop and the
terminal UI can change it, and the phones show it read-only.

## Registering

The App is created from GitHub's manifest flow and its fields are pasted into **Settings → GitHub
App** on the computer: the numeric App id, the private key (a PEM file) and a webhook secret of at
least 16 visible characters. The host validates each one, asks GitHub for the installations, and
only then saves the App. Nothing is written on a refusal.

The private key and the webhook secret go to the encrypted vault. The settings page and every
client show the App id and the installations, never the secrets.

The manifest asks for the minimum permissions the events need: **Contents: Read**, **Pull requests:
Read** and **Checks: Read**, and subscribes to **pull_request**, **pull_request_review**,
**check_run** and **issues**. [Setting one up](guides/github-app-manifest.md) walks through the
manifest and the webhook URL.

## Installations and mapping

An App is installed on an organization or a personal account, and the person chooses which of its
repositories the installation may reach. The host keeps the installations and their repositories as
GitHub reports them; **Refresh installations** reads them again, and drops any mapping whose
repository no installation carries any more.

Each repository may map to one or more bots and, optionally, to branches. An empty branch list
means every branch. The mapping is stored with the App, so a delivery can filter to the mapped bots'
own event triggers and to the branch the event names.

## Deliveries

GitHub sends each delivery to `POST /hooks/github` with an HMAC-SHA256 signature over the body
(`X-Hub-Signature-256`). The host verifies it with the App's webhook secret before reading the
payload; an unsigned or wrongly signed body is refused. The `github` hook id reuses the relay's
public hook path, the same one routine webhooks use, so a private relay gets the same route.

A delivery id (`X-GitHub-Delivery`) is remembered for ten minutes: a replay inside that window
answers `{"duplicate": true}` and queues nothing. Outside the window it is new again. Bodies over
1 MiB and malformed ids are refused.

Only the four subscribed events reach routines. A delivery of anything else, from an unmapped
repository, or on a branch the mapping excludes is ignored. A passing delivery queues the matching
event-trigger routines of the mapped bots only, carrying the delivery id and the repository in the
run's event, so the run can be deduplicated too.

## Tokens

Installation tokens are minted on demand from the App's JWT and cached until shortly before they
expire, then renewed. A token is requested only for an installation the App knows.

## Where it shows

**Settings → GitHub App** on the desktop holds the whole flow: register or replace the App, open the
manifest, install it on repositories, refresh installations, map a repository to bots (with
branches), unmap it, and disconnect the App. The terminal UI's Channels sheet and the iPhone and
Android Channels cards show the connected App, its installations and its mappings read-only, and
point back to the desktop for changes.

## API

`githubAppStatus`, `setGitHubApp`, `disconnectGitHubApp`, `githubInstallations`, `mapGitHubRepo`
and `unmapGitHubRepo`. Each returns the fresh status. `POST /hooks/github` receives the deliveries.
