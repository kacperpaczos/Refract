# Preset Architecture for Reliable Desktop Layouts

## Why this matters

The project should not try to manage the desktop as one opaque thing. GNOME desktop behavior is split across layers with very different stability guarantees.

For this reason, the application should treat desktop presets as an orchestration problem, not as a collection of ad-hoc tweaks.

The main value of the project is not only "setting a dock" or "enabling a panel". The real value is that a preset:

- is predictable
- can be applied in one action
- does not break the session
- can be rolled back
- can explain why a layout cannot be applied fully

## What we learned

### 1. There is no single stable GNOME Shell API for deep shell customization

There is no official, stable middleware layer for modifying GNOME Shell layouts in a way that replaces extensions entirely.

That means features like:

- dock
- panel
- menu
- tiling
- workspace switcher
- desktop icons

still depend on GNOME Shell extensions or on vendor-specific downstream integrations.

### 2. Desktop integration and shell customization are different problems

For desktop integration, there are shared APIs such as:

- `gsettings`
- `dconf`
- D-Bus
- `xdg-desktop-portal` for app-level desktop interaction

These are good foundations for reliable presets.

For shell customization, however, the project still has to deal with:

- extensions
- extension compatibility
- vendor forks
- shell reloads
- conflicts between layout providers

### 3. Vendors use mixed strategies

Ubuntu, Zorin, and similar distributions do not follow one universal approach.

In practice:

- some extensions are upstream
- some are vendor-maintained forks
- some are vendor-owned integrations
- some are repackaged upstream projects with distro-specific compatibility work

This means the project cannot assume that one layout concept maps cleanly to one universal extension set across all systems.

## Architectural implication

The program should be designed as an orchestrator plus validator, not as a "magic installer".

Its job is to:

- detect the desktop environment and shell version
- detect which extensions are available
- detect whether an extension is upstream or vendor-specific
- apply stable settings through `gsettings` and `dconf`
- enable only the required extensions
- disable conflicting extensions
- validate the result
- offer rollback when something fails

## Recommended model

Presets should be defined as structured, inspectable artifacts with explicit compatibility and recovery metadata.

Each preset should include:

- `base_settings`
- `required_extensions`
- `extension_config`
- `conflicting_extensions`
- `compatibility_rules`
- `health_checks`
- `rollback_plan`

## Three layers of a preset

### 1. Shell/core layer

This is the most stable layer and should be the foundation of every preset.

Examples:

- wallpaper
- theme
- fonts
- window button layout
- workspace behavior
- favorite apps
- autostart defaults
- general GNOME preferences

These should be applied through:

- `gsettings`
- `dconf`

### 2. Extensions layer

This layer provides layout-specific UX behavior.

Examples:

- dock
- panel
- application menu
- tiling
- workspace indicator
- desktop icons

This layer is inherently less stable. Because of that, presets should treat extensions as dependencies of a layout, not as the foundation of the desktop profile.

That means:

- the preset declares which extensions it needs
- the preset declares which extensions conflict
- the preset declares version compatibility
- the preset can fall back to degraded behavior if an extension is missing or unsupported

### 3. Vendor/profile layer

The same conceptual layout may need different implementations depending on the platform.

For example, "traditional desktop" may map to:

- upstream GNOME extension set
- Ubuntu-like implementation
- Zorin-like implementation
- Manjaro-like implementation

Because of that, presets should support variants such as:

- `traditional/upstream-gnome`
- `traditional/ubuntu-like`
- `traditional/zorin-like`

## Why the current `.de` layout direction is good

The existing layout files already move in the right direction:

- they describe required steps declaratively
- they separate extension installation from configuration
- they disable known conflicts
- they model desktop setup as a sequence of actions

This is a strong base for a production-ready preset engine.

## What should be added next

To turn the current layout approach into a robust preset system, the next iteration should add:

- explicit `compatibility` metadata
- `provider` or `variant` metadata
- preflight validation
- post-apply health checks
- rollback snapshots
- degraded-mode execution when some dependencies are unavailable

## Suggested execution flow

A reliable preset engine should follow this order:

1. Detect environment
2. Validate shell version and provider
3. Check required tools and extension availability
4. Save rollback snapshot
5. Apply core settings
6. Install or enable required extensions
7. Apply extension configuration
8. Disable conflicting extensions
9. Reload shell only when needed
10. Run health checks
11. Roll back or warn if the final state is degraded

## Product goal

The goal of the project should be:

"Create desktop presets that preserve desktop quality and make deployment easy, predictable, and reversible."

That means the application should optimize for:

- consistency
- safety
- explainability
- compatibility
- reversibility

not only for how many tweaks it can apply.

## Strategic assessment of the pivot

The current direction based on GNOME extensions and direct `dconf` manipulation is a valid prototype path, but it is a weak product foundation.

It is fragile because it operates on unstable implementation details rather than on a stable contract.

The proposed pivot is technically sound if it is framed correctly:

- not as "one universal Linux desktop layout system"
- but as "a declarative UX intent layer with native backend adapters"

This distinction is critical.

### What is good about the new direction

The pivot moves the project from:

- hacking shell internals
- coupling layouts to specific extensions
- accumulating backend-specific technical debt

toward:

- describing intent
- validating capabilities
- planning execution
- applying native configuration through adapters

This is a much stronger long-term architecture.

## Important correction: desktops are not only rendering engines

GNOME, KDE, and similar environments are not passive renderers. They are full UX platforms with their own semantics, capabilities, and design philosophies.

Because of that, the project should not assume that all desktop concepts can be translated losslessly across environments.

The correct target is:

- portable intent
- partially portable behavior
- explicitly non-portable implementation details

The project should aim to standardize outcomes and workflow goals, not force identical visual structures everywhere.

## What the spec should describe

The UX specification should describe:

- workflow goals
- interaction policies
- user intent
- accessibility and ergonomics constraints
- behavioral priorities
- fallback expectations

The specification should avoid describing:

- pixel-perfect geometry
- desktop-specific shell widgets
- hard assumptions about panel placement semantics
- backend-specific implementation details

Good examples of declarative intent:

- "application launching must be fast and visually discoverable"
- "workspace switching must be keyboard-first"
- "notifications should be quiet during focus mode"
- "active work items must remain visible without entering overview"

Weak examples of overfitted intent:

- "bottom panel, 48 px, menu in the lower-left corner"
- "dock centered with exact icon spacing"

## Product positioning

The strongest product fit is not a generic enthusiast customization tool.

The strongest fit is:

- organizational desktop standardization
- workstation provisioning
- shared environments
- reproducible workflow profiles
- OEM and distribution integration

### Best-fit segments

#### 1. Organizations

This is the strongest deployment target.

Value proposition:

- predictable onboarding
- repeatable desktop policy
- lower support burden
- rollback and auditability
- easier deployment without maintaining a full custom distro

#### 2. Integrators and distribution builders

This is also a strong fit.

Examples:

- Linux OEMs
- educational deployments
- remixes and desktop-focused distributions
- vertical installations

#### 3. Enthusiasts

This is a useful validation community, but a weaker strategic anchor.

Enthusiasts can:

- test edge cases
- contribute presets
- help with compatibility

But they also increase the risk of turning the project into an open-ended customization layer instead of a reliable policy engine.

## Biggest architectural risks

### 1. Over-abstraction

This is the primary risk.

If the project tries to unify concepts that are not semantically equivalent across desktops, the resulting standard will be vague, leaky, and difficult to implement.

### 2. Missing capability negotiation

A preset must never assume a backend can satisfy all requested behavior.

The architecture needs an explicit capability model with concepts such as:

- `supports`
- `requires`
- `conflicts_with`
- `degrades_to`
- `confidence`

Without this, the specification becomes misleading.

### 3. Chasing full portability

Full portability of desktop UX is not a realistic target.

The realistic target is:

- portability of intent
- bounded portability of behavior
- transparent incompatibility where required

### 4. Recreating the current technical debt inside adapters

Adapters can easily become a new hiding place for brittle hacks.

Each backend adapter must clearly classify operations into:

- stable native API
- semi-stable integration
- discouraged workaround
- forbidden hack

If all techniques are treated equally, the project will recreate the same fragility behind a cleaner file format.

### 5. Weak versioning and compatibility contracts

The system needs first-class version awareness for:

- spec version
- adapter version
- backend version
- capability profile
- result status

Without strict compatibility contracts, enterprise use will remain risky.

### 6. Unclear boundary between UX policy and personalization

The project must decide what it is.

Possible identities include:

- policy engine for organizations
- workflow preset engine
- full desktop customization system

Trying to fully optimize for all three at once will dilute both architecture and messaging.

## Recommended architecture for the pivot

The healthiest model is a four-layer system:

1. `UX Spec`
2. `Capability Graph`
3. `Planner`
4. `Adapter`

### 1. UX Spec

This layer defines user intent, workflow expectations, policy, and priorities.

It should remain backend-agnostic wherever possible.

### 2. Capability Graph

This layer describes what a backend can actually do.

It should include:

- supported concepts
- unsupported concepts
- optional enhancements
- degradation paths
- known conflicts

### 3. Planner

This layer is essential and should not be collapsed into the adapter.

Its role is to:

- compare intent with capabilities
- produce an execution plan
- decide whether the result is full, degraded, or blocked
- prepare a rollback strategy

### 4. Adapter

This layer performs the native system changes.

Responsibilities:

- apply settings safely
- execute installation or enablement steps
- perform native translation
- run validation hooks
- report execution status

## What success looks like

A successful system should be able to say:

- "applied fully"
- "applied with degradation"
- "blocked due to missing capability"
- "rolled back successfully"

This is more important than pretending every preset maps perfectly everywhere.

## Recommended MVP scope

The project should not launch with the ambition of becoming a universal Linux UX standard overnight.

A strong MVP would include:

- one spec format
- one clear use case
- two desktop backends
- one compatibility model
- one rollback mechanism

Recommended first use cases:

- `Developer Workstation`
- `Shared Lab Desktop`
- `Creator Workstation`

These are strong because they justify:

- repeatability
- controlled defaults
- workflow-oriented UX
- validation and rollback

## Final verdict

The pivot is directionally correct.

Technically, it is much stronger than continuing to rely on extension-heavy shell hacking.

Product-wise, it has real potential if positioned as:

- a declarative desktop policy and workflow preset system

rather than:

- a promise of perfect cross-desktop visual uniformity

The core principle should be:

"Standardize intent, negotiate capability, execute natively, degrade honestly, and roll back safely."
