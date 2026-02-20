# CloudForge-Style Metals Workflow Demo

This demo uses synthetic metals RFQ data and maps reliability evidence to revenue-ops KPI proxies.

## Run

```bash
scripts/demo/cloudforge_workflow_demo.sh
```

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

- `time_to_first_quote`: infer from transition timestamps in `timetravel.capture`.
- `followup_sla_breach_rate`: infer from cadence/response timing + anomaly windows.
- `stage_leakage_rate`: infer from missing or delayed expected transitions in replay.

## Security and Safety Posture

- Synthetic-only data: no real customer identifiers, credentials, or tokens.
- Demo enforces `--share-safe` for export.
- Demo validates fail-closed behavior by expecting refusal-path exit code `3` on a synthetic refusal fixture.

## Public References

- ASTM A36/A36M overview: https://store.astm.org/a0036_a0036m-19.html
- ICC Academy (Incoterms FCA guidance): https://academy.iccwbo.org/incoterms/article/incoterms-2020-fca-or-fob/
- Example EN 10204 3.1 certificate (includes heat-number style marking):
  https://www.tzb-arseco.cz/user/documents/upload/Potrub%C3%AD%20INOX%20304/Inspek%C4%8Dn%C3%AD%20certifik%C3%A1t%20-%20nerezov%C3%A9%20trubky%2015052024%201.pdf
