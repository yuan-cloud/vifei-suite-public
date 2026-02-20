# CloudForge-Style Metals Workflow Demo

This demo uses synthetic metals RFQ data and maps reliability evidence to revenue-ops KPI proxies.

## Run

```bash
scripts/demo/cloudforge_workflow_demo.sh
```

## Optional Visual Capture

Generate deterministic TUI visuals for presentation screenshots:

```bash
cargo run -p vifei-tui --bin capture_readme_assets
```

Primary output paths:

- `docs/assets/readme/incident-lens-showcase.svg`
- `docs/assets/readme/forensic-lens-showcase.svg`
- `docs/assets/readme/truth-hud-showcase.svg`

## Domain-Specific Payload Syntax Used

The fixture at `fixtures/metals-commercial-workflow.jsonl` models a commercial metals RFQ with:

- `material.standard`: `ASTM A36/A36M`
- `material.form`: `plate`
- `material.thickness_mm`, `width_mm`, `length_mm`
- `material.quantity_tons`
- `delivery.incoterm`: `FCA`
- `delivery.ship_to`, `delivery.need_by`
- `certification.inspection_document`: `EN 10204 3.1`
- `certification.heat_number_required`: `true`

## Why These Fields Are Realistic

- ASTM A36/A36M is a standard for carbon structural steel shapes/plates/bars.
- FCA is an Incoterms 2020 rule commonly used for multimodal/containerized shipping.
- EN 10204 3.1 certificate formats include traceability markings and heat-number references.

## KPI Proxy Mapping

- `kpi_proxy_time_to_first_quote_hours`: computed from fixture transition timestamps (`rfq.intake` -> `quote.commit`).
- `kpi_proxy_followup_sla_breach_rate`: computed from synthetic cadence response windows (demo SLA threshold: 4 hours).
- `kpi_proxy_stage_leakage_rate`: computed from expected-vs-observed transition sequence coverage.

## Security and Safety Posture

- Synthetic-only data: no real customer identifiers, credentials, or tokens.
- Demo enforces `--share-safe` for export.
- Demo validates fail-closed behavior by expecting refusal-path exit code `3` on a metals-specific synthetic refusal fixture.
- Demo prints first blocked pattern and `redacted_match` from `refusal-report.json` as explicit redaction evidence.

## Public References

- [ASTM A36/A36M overview](https://store.astm.org/a0036_a0036m-19.html)
- [ICC Academy Incoterms FCA guidance](https://academy.iccwbo.org/incoterms/article/incoterms-2020-fca-or-fob/)
- [EN 10204 overview](https://en.wikipedia.org/wiki/EN_10204)
