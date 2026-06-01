# Zeno Meeting Pointers and Negotiation Playbook

Internal draft for the founder meeting.

## Meeting Goal

Position MeshInspector as an independently owned manufacturability layer for AI-generated jewelry models.

The goal of the meeting is not to sell source code. The goal is to get agreement on a paid pilot that validates whether Zeno should license the product for commercial use.

## Core Message

Use this framing:

> I built this as a separate manufacturability engine because there is a gap between AI-generated 3D jewelry and production-ready jewelry. The output from image-to-3D can look good, but before printing or casting, users need confidence around size, weight, wall thickness, repair state, hollowing, and export quality. This tool is designed to turn generated models into manufacturing-ready assets.

Then clarify ownership:

> The core product and workflow are mine. It also uses licensed third-party geometry/runtime components under the hood, similar to how professional CAD and manufacturing tools use commercial kernels or SDKs. For commercial deployment, those runtime rights need to be covered, and I can include that inside the onboarding or production license. Source code and internal implementation details are not part of the license.

## Demo Flow

1. Show the problem with a generated jewelry model.
2. Upload/open model in MeshInspector.
3. Show viewport measurements and model metrics.
4. Show manufacturability issues: size, weight, wall thickness, holes, self-intersections, export readiness.
5. Run a repair or make-manufacturable workflow.
6. Show size/weight controls.
7. Show hollowing/wall-thickness tools.
8. Show version history and compare.
9. Download manufacturing STL.
10. End with the commercial proposal, not with technical internals.

## Value Proposition

For Zeno:

- Makes generated jewelry models closer to production-ready output.
- Reduces manual CAD cleanup before 3D printing or casting.
- Helps users target exact ring size, dimensions, and weight class.
- Adds a high-value manufacturing layer to Zeno's image/text-to-3D workflow.
- Can become a premium paid feature or enterprise differentiator.

For you:

- You keep reusable core IP.
- Zeno becomes first commercial partner/licensee.
- You get paid for pilot and productionization.
- You preserve the ability to reuse the same manufacturing engine in your own future startup.

## Recommended Pitch Options

### Option 1: Paid Pilot

Recommended first offer.

- Fee: `INR 7.5L-12L`
- Term: `30-45 days`
- Scope: limited usage, model validation, workflow fit, integration planning
- Includes: evaluation access, production-readiness setup, standard third-party runtime coverage for pilot
- Excludes: source code, exclusivity, unlimited usage, custom enterprise deployment

Suggested wording:

> I suggest we start with a paid 45-day pilot. The pilot validates the real workflow on Zeno-generated models, covers setup and runtime costs, and gives both sides enough data to decide the production license.

### Option 2: Annual Commercial License

Offer after pilot or if they want to move fast.

- Fee: `INR 36L-60L/year`
- License: non-exclusive, non-transferable, no source code
- Scope: Zeno jewelry workflows only
- Includes: hosted access or controlled integration, standard support, standard runtime coverage for agreed usage tier
- Excludes: source ownership, sublicensing, broad exclusivity, private deployment unless priced separately

Suggested wording:

> After the pilot, the clean model is an annual product license. Zeno gets commercial usage rights inside its jewelry workflow, while I retain ownership of the software and continue maintaining it.

### Option 3: Private / On-Prem Deployment

Use only if they require deployment inside Zeno infrastructure.

- Fee: `INR 60L-1.2Cr/year`
- Setup fee: `INR 10L-25L`
- License: binary/container access only, no source code
- Includes: deployment support, enterprise runtime coverage, limited SLA
- Excludes: source code and unrestricted redistribution

Suggested wording:

> A private deployment is possible, but it is a different commercial tier because it changes licensing, infrastructure, support, and compliance obligations.

### Option 4: Perpetual Binary License

Avoid unless they strongly insist.

- Fee: `INR 1Cr+`
- No source code
- Limited to Zeno jewelry product line
- Maintenance/support charged annually at `20-25%`
- No exclusivity unless priced separately

Suggested wording:

> I would recommend annual licensing rather than perpetual, because the tool will keep evolving with new manufacturing constraints and generated-model failure modes.

## Negotiation Guardrails

Do not agree to:

- source code transfer
- unlimited perpetual license at a low price
- broad exclusivity
- "we will decide IP later"
- free commercial pilot
- Zeno ownership of the core tool
- disclosure of vendor names, architecture, or build system unless required for later legal/security review

Acceptable concessions:

- discounted first pilot because Zeno is the first commercial partner
- pilot fee credited partially against year-one license
- limited jewelry-category exclusivity for a high annual fee
- custom Zeno workflow integration as paid services
- direct Zeno procurement of required third-party runtime rights, if needed

## If They Ask For Source Code

Response:

> Source code is not part of this license. The product is broader than one Zeno integration, and I need to retain it for future manufacturing use cases. What I can provide is a production-grade hosted/API or private binary deployment with agreed support and uptime terms.

## If They Ask About Third-Party Components

Response:

> The software includes licensed third-party geometry/runtime components. That is normal for CAD/manufacturing software. I will make sure commercial usage rights are covered for the agreed deployment model. I do not disclose internal vendor names or architecture at the evaluation stage, but we can document third-party compliance obligations during contract review if required.

## If They Ask For Exclusivity

Response:

> I cannot offer broad exclusivity because this is a reusable manufacturing platform. If exclusivity matters, we can discuss a narrow, time-bound exclusivity for jewelry-specific image-to-3D manufacturability inside Zeno, priced separately.

Suggested exclusive pricing:

- Narrow category exclusivity: `INR 75L-2Cr/year`
- Term: maximum `12 months`
- Scope: jewelry image-to-3D manufacturability only
- Explicitly excludes broader manufacturing, CAD repair, 3D printing, industrial, dental, medical, and general mesh workflows

## If They Say Pricing Is High

Response:

> The value is not just software access. It adds a manufacturability layer to Zeno's core output, reduces failed prints and manual CAD cleanup, and can become a premium workflow. I can make the pilot affordable, but production rights, runtime coverage, and IP protection have to be priced properly.

Fallback:

- Pilot: `INR 3L-5L`
- Limits: 30 days, limited models, no integration guarantee, no production use
- Production license remains separate

## Success Metrics For Pilot

Agree these before starting:

- model upload and processing success rate
- ring size and dimension accuracy
- predicted weight accuracy by material
- STL export readiness
- wall-thickness violation reduction
- hollowing and weight reduction quality
- time saved versus manual CAD cleanup
- number of generated models validated

## End The Meeting With A Next Step

Suggested close:

> If this is useful for Zeno, I suggest we structure it as a paid pilot. I can share a short pilot proposal with scope, timeline, commercial terms, and IP boundaries. Once we validate it on real Zeno models, we can convert it into an annual production license.

