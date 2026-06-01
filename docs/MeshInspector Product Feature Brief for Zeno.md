# MeshInspector Product Feature Brief for Zeno

Draft product brief for founder discussion.

## One-Line Summary

MeshInspector turns AI-generated jewelry meshes into measurable, editable, manufacturing-ready 3D assets.

## Problem

Image-to-3D and text-to-3D jewelry generation can produce visually attractive models, but jewelry customers and manufacturers still need to answer practical production questions before printing or casting:

- Is the model the exact intended size?
- Does it match the target ring size or dimensions?
- What will it weigh in gold, silver, platinum, or other materials?
- Is the mesh closed and printable?
- Are there holes, self-intersections, thin areas, or unsafe wall thicknesses?
- Can the model be hollowed to reduce weight while preserving details?
- Can the final STL be trusted for manufacturing?

MeshInspector focuses on this gap between generated geometry and production-ready jewelry.

## Core Capabilities

### Model Ingestion

- Upload generated 3D models.
- Normalize and prepare mesh artifacts for viewing, analysis, editing, and export.
- Maintain original model and derived versions.

### Manufacturability Analysis

- Detect mesh health issues.
- Identify holes, self-intersections, shell issues, and thin regions.
- Estimate production readiness.
- Generate recommendations before export.

### Measurement and Dimension Control

- Show model dimensions in millimeters.
- Detect and display ring-related measurements where applicable.
- Support target ring size workflows.
- Support resizing while preserving important detail regions.

### Material and Weight Prediction

- Estimate weight by material.
- Support common jewelry materials such as gold and silver.
- Help users fit a design into target weight classes.
- Update weight prediction after resizing, hollowing, and mesh edits.

### Repair and Preparation

- Repair mesh issues before manufacturing operations.
- Improve model closure and export-readiness.
- Prepare generated geometry for downstream STL export.

### Hollowing and Wall Thickness

- Create hollow shells to reduce weight.
- Control wall thickness.
- Preserve ornament-heavy regions more carefully than simpler areas.
- Add drain holes when needed for casting/printing workflows.

### Local Editing and Advanced Tools

- Thicken unsafe regions.
- Smooth rough generated surfaces.
- Scoop or recess selected regions where thickness allows.
- Work with semantic jewelry regions such as inner band, outer band, head, and ornament relief.
- Keep a versioned edit history instead of destructively overwriting the model.

### Versioning and Comparison

- Every major edit creates a new version.
- Users can compare versions.
- Users can restore earlier versions as branches.
- Review before/after changes visually and numerically.

### Export

- Export manufacturing-oriented STL.
- Preserve the current selected version when downloading.
- Support iteration from generated model to corrected manufacturing asset.

## Current UI Direction

The interface is being structured like a production CAD/manufacturing workbench:

- central 3D viewport for the model
- top tool groups for repeatable muscle memory
- dedicated tool configuration panel
- model metrics and manufacturability status
- version history and activity tracking
- interactive viewing and inspection tools

The goal is to make the workflow understandable for designers while still giving manufacturing teams enough control.

## Zeno Integration Possibilities

### Hosted Review Tool

Zeno sends generated models into MeshInspector for review and correction.

Best for pilot.

### Embedded Manufacturing Step

MeshInspector becomes a step inside Zeno's generation workflow:

1. generate model
2. inspect manufacturability
3. adjust size/weight/thickness
4. export production STL

Best for production SaaS integration.

### Premium Feature

Zeno can expose this as a paid feature:

- "Manufacturing-ready export"
- "Weight-optimized jewelry model"
- "Casting-ready STL"
- "Production validation report"

## Why This Matters For Zeno

This feature can move Zeno from design generation toward production-ready jewelry workflows.

Potential business value:

- higher trust in generated outputs
- fewer failed prints
- less manual CAD cleanup
- stronger differentiation versus generic image-to-3D tools
- possible premium pricing for manufacturing-ready exports
- better bridge between creative generation and actual jewelry production

## Pilot Scope

Recommended pilot duration: `45 days`

Recommended pilot goals:

- test on real Zeno-generated jewelry models
- validate size and weight accuracy
- validate STL export readiness
- compare repair/hollowing results against manual CAD expectations
- identify failure cases from AI-generated meshes
- define production integration scope

Recommended pilot success metrics:

- percent of uploaded models successfully processed
- dimension accuracy after resizing
- weight prediction accuracy by material
- reduction in wall-thickness violations
- reduction in manual cleanup effort
- successful STL export rate

## Commercial Position

MeshInspector should be offered as a licensed product, not transferred as source code.

Recommended structure:

- paid pilot
- annual commercial license after validation
- no source code transfer
- no broad exclusivity
- commercial runtime and third-party component coverage included in agreed usage tier
- private/on-prem or enterprise deployment priced separately

## Suggested Founder Summary

> Zeno already helps users create jewelry designs from images or prompts. MeshInspector adds the manufacturing bridge: it checks whether those generated models can actually be sized, weighted, hollowed, repaired, and exported for production. I would like to run this as a paid pilot first, validate it on real Zeno outputs, and then convert it into a commercial license if the results are strong.

