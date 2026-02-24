# Aptabase Growth Signal Ops Note

This note documents expected cadence and validation steps for the phase-1 growth analytics signals.

## Event Cadence Expectations

- `feature_used`: one event per successful feature completion (not on key press/release noise).
- `aha_moment_reached`: once per install/profile when successful feature count first reaches `5`.
- `upgrade_prompt_shown`: each time the upsell banner is shown.
- `upgrade_prompt_action`: one per user interaction (`cta_clicked`, `dismissed`, or `closed`).
- `upgrade_checkout_result`: emitted for checkout lifecycle milestones (`started`, optional `completed`/`failed` when integrated).

## Prompt Eligibility Rules

The prompt can be shown only when:

1. `aha_moment_reached` has happened.
2. User is not marked paid (`is_paid = false` in growth state).
3. Onboarding is complete.
4. Last shown timestamp is older than 14 days.

Backend emits `upgrade-prompt-eligible` only when eligible.

## Manual Validation Checklist

1. Perform 5 successful feature completions and confirm `aha_moment_reached` appears once.
2. Confirm the upgrade banner appears after eligibility is reached.
3. Dismiss the banner and confirm it does not reappear immediately.
4. Confirm `upgrade_prompt_shown` and `upgrade_prompt_action` events are visible in Aptabase.
5. Click CTA and confirm `upgrade_checkout_result` with `result=started` and `source=aha_prompt`.
6. Set `share_usage_analytics=false` and confirm no Aptabase events are sent while growth behavior remains stable.
