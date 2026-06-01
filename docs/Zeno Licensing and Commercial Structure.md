# Zeno Licensing and Commercial Structure

Internal commercial and legal draft. This is not legal advice; final terms should be reviewed by a lawyer before signature.

## Recommended Commercial Position

MeshInspector should be licensed to Zeno, not sold.

The preferred structure is:

- non-exclusive commercial license
- no source code
- no IP assignment
- no broad exclusivity
- usage limited to Zeno's jewelry workflow
- third-party runtime/commercial component costs included in package pricing or separately priced as enterprise requirements

## Public Third-Party Licensing Facts

Internal note: MeshLib SDK publicly states that commercial/business use requires a commercial license, and it supports Windows, Linux, macOS, and WebAssembly. Its public license page says pricing is tailored and refers to no royalties or per-unit fees, with startup-program options. The MeshInspector app pricing page is separate from MeshLib SDK licensing and should not be treated as the SDK price.

References:

- MeshLib license page: https://meshlib.io/license/
- MeshLib documentation license page: https://meshlib.io/documentation/License.html
- MeshInspector app pricing page: https://meshinspector.com/pricing/

External wording should avoid naming specific internal vendors unless required:

> The software may incorporate licensed third-party geometry, runtime, cloud, or infrastructure components. Commercial deployment requires that the relevant usage rights are covered for the agreed deployment model and usage tier.

## Ownership Terms

### Background IP

Suggested clause:

> All software, algorithms, source code, workflows, architecture, tooling, know-how, documentation, and technology developed before or independently of the agreement remain the sole property of Licensor.

### Product IP

Suggested clause:

> Licensor retains all right, title, and interest in and to the Software, including all improvements, updates, derivative modules, bug fixes, internal tools, implementation methods, and platform components, except for Licensee-owned input data and exported customer files.

### Customer Outputs

Suggested clause:

> Licensee and its end users retain ownership of uploaded models, generated design inputs, and exported model files produced through authorized use of the Software, subject to the license terms and third-party rights.

## License Grant

Suggested clause:

> Licensor grants Licensee a limited, non-exclusive, non-transferable, non-sublicensable license to access and use the Software solely for Licensee's jewelry design and manufacturability workflow during the applicable term and within the agreed usage tier.

## Source Code Exclusion

Suggested clause:

> No source code, build scripts, internal architecture, vendor list, algorithmic implementation details, training materials, development tools, or unpublished technical documentation are included in the license unless expressly agreed in a separate signed agreement.

## Third-Party Components

Suggested clause:

> The Software may incorporate, link to, or interoperate with third-party libraries, SDKs, runtimes, APIs, cloud services, or infrastructure components. Licensor is responsible for maintaining sufficient rights to provide the Software under the agreed deployment model. Licensee receives no ownership interest or direct license to such third-party components except as necessary to use the Software under this Agreement.

## Additional Third-Party Cost Trigger

Suggested clause:

> If Licensee requests enterprise deployment, on-premise deployment, white-label distribution, high-volume processing, source escrow, separate security review, vendor audit, or usage outside the agreed tier, additional third-party licensing, infrastructure, compliance, or support fees may apply.

## Restrictions

Suggested clause:

> Licensee shall not copy, modify, reverse engineer, decompile, disassemble, resell, sublicense, redistribute, white-label, benchmark publicly, use to build a competing product, or permit unauthorized third-party access to the Software.

## Deployment Models

### Hosted SaaS/API

Recommended for your long-term strategy.

- You host the software.
- Zeno gets access via web app, API, or embedded workflow.
- You control source, runtime, updates, and customer boundaries.
- Pricing can include standard third-party runtime coverage.

Suggested pricing:

- Pilot: `INR 7.5L-12L`
- Production: `INR 36L-60L/year`
- Usage overages: priced by model volume, compute time, or enterprise support tier

### Private Cloud / On-Prem Binary

Use only if necessary.

- You provide containerized/binary deployment.
- No source code.
- Higher price due to support, compliance, runtime, and deployment complexity.

Suggested pricing:

- Setup: `INR 10L-25L`
- Annual license: `INR 60L-1.2Cr/year`
- Maintenance/SLA: included up to defined limits or charged separately

### Perpetual Binary License

Avoid unless strongly required.

Suggested pricing:

- `INR 1Cr+`
- No source code
- One product line only
- Maintenance/support: `20-25%` annually
- Third-party runtime changes billed separately

## Pilot Proposal Terms

Recommended pilot:

- Fee: `INR 7.5L`
- Term: `45 days`
- Scope: limited production-readiness validation on Zeno-generated jewelry models
- Usage: internal evaluation only, no public commercial rollout
- Deliverables:
  - hosted evaluation workspace
  - model upload and manufacturability checks
  - repair/resize/hollow/export workflows
  - pilot report with success metrics and production recommendation
- Exclusions:
  - source code
  - vendor disclosure
  - exclusivity
  - unlimited models
  - on-prem deployment
  - production SLA

## Annual Production Proposal

Recommended year-one license:

- Fee: `INR 36L/year`
- License: non-exclusive
- Deployment: hosted SaaS/API or controlled embedded workflow
- Scope: Zeno jewelry models only
- Includes:
  - commercial usage rights for agreed tier
  - standard runtime coverage
  - bug fixes and minor improvements
  - limited monthly support hours
- Excludes:
  - source code
  - on-prem/private deployment
  - broad exclusivity
  - custom feature development beyond agreed scope

## Custom Development

Suggested terms:

- Custom integration: `INR 1.5L-3L/week`
- New feature development: fixed-scope quote
- Ownership:
  - default: you own improvements to the core platform
  - Zeno receives usage rights under their license
  - Zeno owns only their confidential inputs, brand assets, and customer data

## Exclusivity

Default position: no exclusivity.

If required:

- only narrow category exclusivity
- max 12 months
- explicit carve-outs for your own startup and broader manufacturing use cases
- separate premium fee

Suggested clause:

> Any exclusivity must be expressly stated in writing, limited by field, geography, duration, and product line, and shall not restrict Licensor from developing, licensing, or operating manufacturing, mesh repair, CAD automation, 3D printing, industrial, dental, medical, or general-purpose geometry tools outside the expressly defined exclusive field.

## Recommended Ask

Lead with:

- `INR 7.5L` paid pilot for 45 days
- pilot converts into `INR 36L/year` annual commercial license
- no source code
- third-party runtime coverage included for agreed pilot/production tier
- enterprise/private/high-volume use priced separately

